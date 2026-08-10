pub mod player;

pub use player::{
    player_classes_handler, player_create_handler, player_list_handler, player_select_handler,
};

use serde::Deserialize;

use crate::game::Interaction;

#[derive(Deserialize)]
pub struct SendInteractionRequest {
    pub client_id: String,
    pub interaction: Interaction,
}

pub async fn send_interaction_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendInteractionRequest>,
) -> StatusCode {
    tracing::info!(client_id = %req.client_id, "POST /interactions");
    let players = state.game_state.active_players.read().await;
    let player = match players.get(&req.client_id) {
        Some(p) => p.clone(),
        None => return StatusCode::NOT_FOUND,
    };
    drop(players);
    state
        .game_state
        .mailboxes
        .push(player.entity_id, req.interaction)
        .await;
    StatusCode::OK
}

use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use tracing::info;

use super::state::{
    AppState, ConnectedClient, GuardedStream, PingBody, SessionEndBody, SessionStartBody,
    SseCleanupGuard, SseQuery, queue_player_disconnected,
};
use std::sync::atomic::Ordering;

use crate::network::event::{
    NetworkEvent, ServerInfoResponse, SessionStartResponse, ThemeInfo, ThemeListResponse,
    ThemeStyleInfo,
};
use crate::network::session::ConnectionKey;

pub async fn server_info_handler(State(state): State<Arc<AppState>>) -> Json<ServerInfoResponse> {
    info!("GET /server/info");
    Json(ServerInfoResponse {
        server_id: state.server_session.id.clone(),
    })
}

pub async fn theme_list_handler(State(state): State<Arc<AppState>>) -> Json<ThemeListResponse> {
    info!("GET /themes/list");
    let themes = state
        .game_state
        .themes
        .values()
        .map(|theme| ThemeInfo {
            id: theme.id.clone().unwrap_or_default(),
            styles: theme
                .styles
                .iter()
                .map(|(key, style)| {
                    (
                        key.clone(),
                        ThemeStyleInfo {
                            fg: style.fg.clone(),
                            bg: style.bg.clone(),
                            modifiers: style.modifiers.clone(),
                        },
                    )
                })
                .collect(),
        })
        .collect();
    Json(ThemeListResponse { themes })
}

pub async fn sse_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SseQuery>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    info!(client_id = %query.client_id, "GET /events - client subscribed to SSE");
    let (personal_tx, personal_rx) = tokio::sync::mpsc::channel::<NetworkEvent>(128);
    let seq = state.next_connection_seq.fetch_add(1, Ordering::Relaxed);
    state.connections.write().await.insert(
        query.client_id.clone(),
        ConnectedClient {
            last_ping: Instant::now(),
            personal_tx,
            seq,
        },
    );
    let inner = ReceiverStream::new(personal_rx).filter_map(|event| {
        serde_json::to_string(&event)
            .ok()
            .map(|data| Ok(Event::default().data(data)))
    });
    let guard = SseCleanupGuard {
        client_id: query.client_id,
        connections: state.connections.clone(),
        game_state: state.game_state.clone(),
        seq,
    };
    let stream = GuardedStream {
        inner,
        _guard: guard,
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub async fn session_start_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SessionStartBody>,
) -> Result<Json<SessionStartResponse>, (StatusCode, String)> {
    let client_id = body
        .client_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let machine_id = ConnectionKey::parse(&client_id)
        .map(|k| k.machine_id)
        .unwrap_or_else(|| client_id.clone());
    let limit = state.game_state.mud_config.max_clients_per_machine;
    let count = state
        .connections
        .read()
        .await
        .keys()
        .filter(|k| k.starts_with(&machine_id))
        .count();
    if count >= limit {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            format!("Connection limit reached for this machine (max {limit})"),
        ));
    }

    info!(client_id = %client_id, "POST /session/start");
    Ok(Json(SessionStartResponse {
        client_id,
        server_id: state.server_session.id.clone(),
    }))
}

pub async fn ping_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PingBody>,
) -> &'static str {
    tracing::debug!(client_id = %body.client_id, "POST /ping");
    let personal_tx = {
        let mut conns = state.connections.write().await;
        conns.get_mut(&body.client_id).map(|client| {
            client.last_ping = Instant::now();
            client.personal_tx.clone()
        })
    };
    if let Some(tx) = personal_tx {
        let _ = tx.send(NetworkEvent::Pong).await;
    }
    "ok"
}

pub async fn session_end_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SessionEndBody>,
) -> &'static str {
    let client_id = &body.session_id;
    let entity_id = queue_player_disconnected(&state.game_state, client_id).await;
    let removed_connection = state.connections.write().await.remove(client_id).is_some();
    info!(
        client_id = %client_id,
        entity_id,
        removed_connection,
        "player disconnected explicitly (session end)"
    );
    "ok"
}

pub async fn maps_reload_handler(State(state): State<Arc<AppState>>) -> &'static str {
    info!("POST /maps/reload");
    state
        .game_state
        .reload_pending
        .store(true, Ordering::Release);
    "ok"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::GameState;
    use crate::game::component::Location;
    use crate::game::entity::character::{Character, CharacterType};
    use crate::game::interaction;
    use crate::game::player::Player;
    use crate::network::session::ServerSession;
    use crate::persistence::Database;
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    async fn battle_app_state(client_id: &str) -> Arc<AppState> {
        let game_state = Arc::new(GameState::load(None).unwrap());
        {
            let mut entities = game_state.active_characters.write().await;
            entities.insert(1, Character::new(1, CharacterType::Player, test_location()));
            entities.insert(2, Character::new(2, CharacterType::Enemy, test_location()));
        }
        game_state.active_players.write().await.insert(
            client_id.to_string(),
            Player {
                id: 1,
                client_id: client_id.to_string(),
                name: "Hero".to_string(),
                entity_id: 1,
            },
        );
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);
        participants.insert("enemy".to_string(), vec![2]);
        game_state
            .engagements
            .add_battle(
                "r".to_string(),
                vec!["player".to_string(), "enemy".to_string()],
                participants,
            )
            .await;

        Arc::new(AppState {
            server_session: ServerSession {
                id: "srv".to_string(),
                name: None,
            },
            game_state,
            db: Database::connect_in_memory().await.unwrap(),
            connections: Arc::new(RwLock::new(HashMap::new())),
            next_connection_seq: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        })
    }

    #[tokio::test]
    async fn session_end_handler_concludes_battle_for_last_departing_faction_member() {
        let state = battle_app_state("m:1").await;

        session_end_handler(
            State(state.clone()),
            Json(SessionEndBody {
                session_id: "m:1".to_string(),
            }),
        )
        .await;

        // session_end_handler only queues the disconnect; the game loop's interaction
        // processing is what actually tears the battle down.
        interaction::process(&state.game_state, &state.db, 0).await;

        assert_eq!(
            state
                .game_state
                .engagements
                .battles
                .find_for_entity(2)
                .await,
            None,
            "battle should conclude once the only player-faction member disconnects"
        );
    }
}
