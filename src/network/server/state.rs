use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use crate::game::GameState;
use crate::network::event::NetworkEvent;
use crate::persistence::Database;
use crate::session::ServerSession;
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
    pub config_path: Option<PathBuf>,
}

// --- SSE disconnect guard ---

pub struct SseCleanupGuard {
    pub client_id: String,
    pub connections: Arc<RwLock<HashMap<String, ConnectedClient>>>,
}

impl Drop for SseCleanupGuard {
    fn drop(&mut self) {
        let client_id = self.client_id.clone();
        let connections = self.connections.clone();
        tokio::spawn(async move {
            connections.write().await.remove(&client_id);
            info!(client_id = %client_id, "SSE disconnected — session ended");
        });
    }
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
