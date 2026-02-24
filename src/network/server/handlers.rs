use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;

use super::state::{
    AppState, ConnectedClient, GuardedStream, PingBody, SessionEndBody, SessionStartBody,
    SseCleanupGuard, SseQuery,
};
use crate::network::event::{NetworkEvent, ServerInfoResponse, SessionStartResponse};

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
    let rx = state.tx.subscribe();
    let inner = BroadcastStream::new(rx).filter_map(|result| {
        result.ok().and_then(|event| {
            serde_json::to_string(&event)
                .ok()
                .map(|data| Ok(Event::default().data(data)))
        })
    });
    let guard = SseCleanupGuard {
        client_id: query.client_id,
        connections: state.connections.clone(),
        tx: state.tx.clone(),
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
) -> Json<SessionStartResponse> {
    let client_id = body
        .client_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    info!(client_id = %client_id, "POST /session/start");
    state.connections.write().await.insert(
        client_id.clone(),
        ConnectedClient {
            last_ping: Instant::now(),
        },
    );
    let _ = state.tx.send(NetworkEvent::StartSession {
        session_id: client_id.clone(),
    });
    Json(SessionStartResponse {
        client_id,
        server_id: state.server_session.id.clone(),
    })
}

pub async fn ping_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PingBody>,
) -> &'static str {
    info!(client_id = %body.client_id, "POST /ping");
    if let Some(client) = state.connections.write().await.get_mut(&body.client_id) {
        client.last_ping = Instant::now();
    }
    let _ = state.tx.send(NetworkEvent::Pong);
    "ok"
}

pub async fn session_end_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SessionEndBody>,
) -> &'static str {
    info!(session_id = %body.session_id, "POST /session/end");
    state.connections.write().await.remove(&body.session_id);
    let _ = state.tx.send(NetworkEvent::EndSession {
        session_id: body.session_id,
    });
    "ok"
}
