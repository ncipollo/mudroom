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
    SseCleanupGuard, SseQuery,
};
use std::sync::atomic::Ordering;

use crate::network::event::{NetworkEvent, ServerInfoResponse, SessionStartResponse};
use crate::network::session::ConnectionKey;

pub async fn server_info_handler(State(state): State<Arc<AppState>>) -> Json<ServerInfoResponse> {
    info!("GET /server/info");
    Json(ServerInfoResponse {
        server_id: state.server_session.id.clone(),
    })
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
    let entity_id = state
        .game_state
        .active_players
        .read()
        .await
        .get(client_id)
        .map(|p| p.entity_id);
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
