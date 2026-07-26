use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use crate::game::GameState;
use crate::game::component::Interaction;
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
    /// Monotonic sequence number identifying this specific SSE connection. A reconnect under
    /// the same client_id supersedes the previous entry, letting stale cleanup guards detect
    /// that their connection has been replaced.
    pub seq: u64,
}

#[derive(Clone)]
pub struct AppState {
    pub server_session: ServerSession,
    pub game_state: Arc<GameState>,
    pub db: Database,
    pub connections: Arc<RwLock<HashMap<String, ConnectedClient>>>,
    pub next_connection_seq: Arc<std::sync::atomic::AtomicU64>,
}

// --- SSE disconnect guard ---

pub struct SseCleanupGuard {
    pub client_id: String,
    pub connections: Arc<RwLock<HashMap<String, ConnectedClient>>>,
    pub game_state: Arc<GameState>,
    pub seq: u64,
}

impl Drop for SseCleanupGuard {
    fn drop(&mut self) {
        let client_id = self.client_id.clone();
        let connections = self.connections.clone();
        let game_state = self.game_state.clone();
        let seq = self.seq;
        tokio::spawn(async move {
            {
                let mut conns = connections.write().await;
                match conns.get(&client_id) {
                    Some(current) if current.seq == seq => {
                        conns.remove(&client_id);
                    }
                    _ => {
                        info!(client_id = %client_id, "SSE disconnected — superseded by a newer connection, skipping cleanup");
                        return;
                    }
                }
            }
            info!(client_id = %client_id, "SSE disconnected — queuing cleanup");
            let entity_id = {
                let players = game_state.active_players.read().await;
                players.get(&client_id).map(|p| p.entity_id)
            };
            if let Some(entity_id) = entity_id {
                // Captured now, as early as possible after deciding to queue the disconnect —
                // not right before the push below — so a reactivation racing this cleanup task
                // is reflected as a bumped epoch by the time dispatch checks it, rather than
                // this stale disconnect picking up the reactivation's own epoch and looking
                // fresh again.
                let epoch = game_state.current_activation_epoch(entity_id).await;
                info!(
                    entity_id,
                    client_id = %client_id,
                    seq,
                    epoch,
                    "queuing PlayerDisconnected"
                );
                game_state
                    .mailboxes
                    .push(
                        entity_id,
                        Interaction::PlayerDisconnected { client_id, epoch },
                    )
                    .await;
            }
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
    #[serde(default)]
    pub class_id: Option<String>,
}

#[derive(Deserialize)]
pub struct PlayerSelectBody {
    pub client_id: String,
    pub player_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Interaction;
    use crate::game::player::Player;
    use std::time::Duration;

    fn connected_client(seq: u64) -> (ConnectedClient, mpsc::Receiver<NetworkEvent>) {
        let (personal_tx, rx) = mpsc::channel(1);
        (
            ConnectedClient {
                last_ping: Instant::now(),
                personal_tx,
                seq,
            },
            rx,
        )
    }

    async fn register_player(game_state: &GameState, client_id: &str) {
        game_state.active_players.write().await.insert(
            client_id.to_string(),
            Player {
                id: 1,
                client_id: client_id.to_string(),
                name: "Hero".to_string(),
                entity_id: 1,
            },
        );
    }

    #[tokio::test]
    async fn superseded_guard_keeps_new_connection_and_skips_disconnect() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let connections = Arc::new(RwLock::new(HashMap::new()));
        register_player(&game_state, "m:1").await;
        let (client, _rx) = connected_client(2);
        connections.write().await.insert("m:1".to_string(), client);

        drop(SseCleanupGuard {
            client_id: "m:1".to_string(),
            connections: connections.clone(),
            game_state: game_state.clone(),
            seq: 1,
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(
            connections.read().await.contains_key("m:1"),
            "newer connection must not be removed by a stale guard"
        );
        assert!(
            game_state.mailboxes.drain(1).await.is_empty(),
            "stale guard must not queue PlayerDisconnected"
        );
    }

    #[tokio::test]
    async fn current_guard_removes_connection_and_queues_disconnect() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let connections = Arc::new(RwLock::new(HashMap::new()));
        register_player(&game_state, "m:1").await;
        let (client, _rx) = connected_client(1);
        connections.write().await.insert("m:1".to_string(), client);

        drop(SseCleanupGuard {
            client_id: "m:1".to_string(),
            connections: connections.clone(),
            game_state: game_state.clone(),
            seq: 1,
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(!connections.read().await.contains_key("m:1"));
        let queued = game_state.mailboxes.drain(1).await;
        assert!(
            queued.iter().any(|i| matches!(
                i,
                Interaction::PlayerDisconnected { client_id, .. } if client_id == "m:1"
            )),
            "expected PlayerDisconnected for m:1, got {queued:?}"
        );
    }
}
