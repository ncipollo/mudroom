use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::Serialize;
use tokio::sync::oneshot;
use tracing::info;

use crate::network::server::state::AppState;

#[derive(Serialize)]
pub struct DebugTickResponse {
    pub tick: u64,
}

/// Runs exactly one tick of the game loop on demand, out of band from the real-time ticker.
/// Only registered when the server is started with `--enable-test-api` — see
/// `AppState::test_api_enabled`. Requests the tick from the dedicated game-loop thread rather
/// than calling `game_loop::run_once` here directly — that can hold a non-`Send` Boa `Context`
/// across an await, which an axum handler's future can't.
pub async fn debug_tick_handler(State(state): State<Arc<AppState>>) -> Json<DebugTickResponse> {
    let (reply_tx, reply_rx) = oneshot::channel();
    state
        .debug_tick_tx
        .send(reply_tx)
        .await
        .expect("game loop thread should still be running");
    let tick = reply_rx.await.expect("game loop thread dropped the reply");
    info!(tick, "POST /debug/tick");
    Json(DebugTickResponse { tick })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::GameState;
    use crate::game::component::{Interaction, Location};
    use crate::game::entity::character::{Character, CharacterType};
    use crate::game::player::Player;
    use crate::network::session::ServerSession;
    use crate::persistence::Database;
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    async fn test_app_state() -> Arc<AppState> {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let db = Database::connect_in_memory().await.unwrap();
        let debug_tick_tx =
            crate::network::server::game_loop_thread::spawn(game_state.clone(), db.clone())
                .unwrap();
        Arc::new(AppState {
            server_session: ServerSession {
                id: "srv".to_string(),
                name: None,
            },
            game_state,
            db,
            connections: Arc::new(RwLock::new(HashMap::new())),
            next_connection_seq: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            test_api_enabled: true,
            debug_tick_tx,
        })
    }

    #[tokio::test]
    async fn debug_tick_handler_advances_tick_each_call() {
        let state = test_app_state().await;

        let first = debug_tick_handler(State(state.clone())).await;
        let second = debug_tick_handler(State(state.clone())).await;

        assert_eq!(second.0.tick, first.0.tick + 1);
    }

    #[tokio::test]
    async fn debug_tick_handler_processes_queued_interaction() {
        let state = test_app_state().await;
        let location = Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        };
        state
            .game_state
            .active_characters
            .write()
            .await
            .insert(1, Character::new(1, CharacterType::Player, location));
        state.game_state.active_players.write().await.insert(
            "m:1".to_string(),
            Player {
                id: 1,
                client_id: "m:1".to_string(),
                name: "Hero".to_string(),
                entity_id: 1,
            },
        );
        state.game_state.mailboxes.push(1, Interaction::Look).await;

        let _ = debug_tick_handler(State(state.clone())).await;

        assert!(
            state.game_state.mailboxes.drain(1).await.is_empty(),
            "queued interaction should be drained by the forced tick"
        );
    }
}
