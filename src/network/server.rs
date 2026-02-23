use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::stream::Stream;
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use tracing::info;

use super::event::NetworkEvent;

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<NetworkEvent>,
}

#[derive(Deserialize)]
struct SessionBody {
    session_id: String,
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    info!("GET /events - client subscribed to SSE");
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| {
        result.ok().and_then(|event| {
            serde_json::to_string(&event)
                .ok()
                .map(|data| Ok(Event::default().data(data)))
        })
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn ping_handler(State(state): State<Arc<AppState>>) -> &'static str {
    info!("POST /ping");
    let _ = state.tx.send(NetworkEvent::Pong);
    "ok"
}

async fn session_start_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SessionBody>,
) -> &'static str {
    info!(session_id = %body.session_id, "POST /session/start");
    let _ = state.tx.send(NetworkEvent::StartSession {
        session_id: body.session_id,
    });
    "ok"
}

async fn session_end_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SessionBody>,
) -> &'static str {
    info!(session_id = %body.session_id, "POST /session/end");
    let _ = state.tx.send(NetworkEvent::EndSession {
        session_id: body.session_id,
    });
    "ok"
}

pub async fn start() -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let (tx, _) = broadcast::channel::<NetworkEvent>(64);
    let state = Arc::new(AppState { tx });

    let app = Router::new()
        .route("/events", get(sse_handler))
        .route("/ping", post(ping_handler))
        .route("/session/start", post(session_start_handler))
        .route("/session/end", post(session_end_handler))
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:0").await?;
    let addr = listener.local_addr()?;

    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    Ok(addr)
}
