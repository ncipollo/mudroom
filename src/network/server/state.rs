use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use crate::game::GameState;
use crate::game::engagement::battle;
use crate::network::event::NetworkEvent;
use crate::network::session::ServerSession;
use crate::persistence::Database;
use futures_util::stream::Stream;
use serde::Deserialize;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tracing::info;

#[derive(Clone)]
pub struct ConnectedClient {
    pub last_ping: Instant,
    pub personal_tx: mpsc::Sender<NetworkEvent>,
}

#[derive(Clone)]
pub struct AppState {
    pub server_session: ServerSession,
    pub game_state: Arc<GameState>,
    pub db: Database,
    pub connections: Arc<RwLock<HashMap<String, ConnectedClient>>>,
}

// --- SSE disconnect guard ---

pub struct SseCleanupGuard {
    pub client_id: String,
    pub connections: Arc<RwLock<HashMap<String, ConnectedClient>>>,
    pub game_state: Arc<GameState>,
}

impl Drop for SseCleanupGuard {
    fn drop(&mut self) {
        let client_id = self.client_id.clone();
        let connections = self.connections.clone();
        let game_state = self.game_state.clone();
        tokio::spawn(async move {
            connections.write().await.remove(&client_id);
            info!(client_id = %client_id, "SSE disconnected — session ended");
            cleanup_player_battle(&game_state, &client_id).await;
            game_state.active_players.write().await.remove(&client_id);
        });
    }
}

async fn cleanup_player_battle(game_state: &GameState, client_id: &str) {
    let entity_id = {
        let players = game_state.active_players.read().await;
        let Some(player) = players.get(client_id) else {
            return;
        };
        player.entity_id
    };

    if let Some((engagement_id, surviving)) =
        battle::participants::remove_entity(&game_state.engagements.battles, entity_id).await
        && surviving <= 1
    {
        game_state.engagements.battles.conclude(engagement_id).await;
        game_state.engagements.battles.remove(engagement_id).await;
    }

    game_state.active_entities.write().await.remove(&entity_id);
}

// Stream wrapper that keeps the guard alive until the stream is dropped.
pub struct GuardedStream<S> {
    pub inner: S,
    pub _guard: SseCleanupGuard,
}

impl<S: Stream + Unpin> Stream for GuardedStream<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

// --- Request body structs ---

#[derive(Deserialize)]
pub struct SessionStartBody {
    pub client_id: Option<String>,
}

#[derive(Deserialize)]
pub struct PingBody {
    pub client_id: String,
}

#[derive(Deserialize)]
pub struct SessionEndBody {
    pub session_id: String,
}

#[derive(Deserialize)]
pub struct SseQuery {
    pub client_id: String,
}

#[derive(Deserialize)]
pub struct PlayerListBody {
    pub client_id: String,
}

#[derive(Deserialize)]
pub struct PlayerCreateBody {
    pub client_id: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct PlayerSelectBody {
    pub client_id: String,
    pub player_id: i64,
}
