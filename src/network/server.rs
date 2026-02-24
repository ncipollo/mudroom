use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::stream::Stream;
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;

use super::event::{NetworkEvent, SessionStartResponse};
use crate::session::ServerSession;

#[derive(Clone)]
struct ConnectedClient {
    last_ping: Instant,
}

#[derive(Clone)]
struct AppState {
    server_session: ServerSession,
    tx: broadcast::Sender<NetworkEvent>,
    connections: Arc<RwLock<HashMap<String, ConnectedClient>>>,
}

// --- SSE disconnect guard ---

struct SseCleanupGuard {
    client_id: String,
    connections: Arc<RwLock<HashMap<String, ConnectedClient>>>,
    tx: broadcast::Sender<NetworkEvent>,
}

impl Drop for SseCleanupGuard {
    fn drop(&mut self) {
        let client_id = self.client_id.clone();
        let connections = self.connections.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            connections.write().await.remove(&client_id);
            let _ = tx.send(NetworkEvent::EndSession {
                session_id: client_id.clone(),
            });
            info!(client_id = %client_id, "SSE disconnected — session ended");
        });
    }
}

// Stream wrapper that keeps the guard alive until the stream is dropped.
struct GuardedStream<S> {
    inner: S,
    _guard: SseCleanupGuard,
}

impl<S: Stream + Unpin> Stream for GuardedStream<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

// --- Handlers ---

#[derive(Deserialize)]
struct SessionStartBody {
    client_id: Option<String>,
}

#[derive(Deserialize)]
struct PingBody {
    client_id: String,
}

#[derive(Deserialize)]
struct SessionEndBody {
    session_id: String,
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    info!("GET /events - client subscribed to SSE");
    let rx = state.tx.subscribe();
    let inner = BroadcastStream::new(rx).filter_map(|result| {
        result.ok().and_then(|event| {
            serde_json::to_string(&event)
                .ok()
                .map(|data| Ok(Event::default().data(data)))
        })
    });
    let guard = SseCleanupGuard {
        client_id: String::new(),
        connections: state.connections.clone(),
        tx: state.tx.clone(),
    };
    let stream = GuardedStream {
        inner,
        _guard: guard,
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn session_start_handler(
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

async fn ping_handler(
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

async fn session_end_handler(
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

// --- Ping reaper ---

async fn run_ping_reaper(
    connections: Arc<RwLock<HashMap<String, ConnectedClient>>>,
    tx: broadcast::Sender<NetworkEvent>,
) {
    let timeout = std::time::Duration::from_secs(30);
    let interval = std::time::Duration::from_secs(10);
    loop {
        tokio::time::sleep(interval).await;
        let now = Instant::now();
        let stale: Vec<String> = connections
            .read()
            .await
            .iter()
            .filter(|(_, c)| now.duration_since(c.last_ping) > timeout)
            .map(|(id, _)| id.clone())
            .collect();
        if !stale.is_empty() {
            let mut guard = connections.write().await;
            for id in stale {
                guard.remove(&id);
                let _ = tx.send(NetworkEvent::EndSession {
                    session_id: id.clone(),
                });
                info!(client_id = %id, "Ping reaper removed stale client");
            }
        }
    }
}

// --- Public API ---

pub async fn start(
    server_session: ServerSession,
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let (tx, _) = broadcast::channel::<NetworkEvent>(64);
    let connections: Arc<RwLock<HashMap<String, ConnectedClient>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let state = Arc::new(AppState {
        server_session,
        tx: tx.clone(),
        connections: connections.clone(),
    });

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

    tokio::spawn(run_ping_reaper(connections, tx));

    Ok(addr)
}
