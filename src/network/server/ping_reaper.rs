use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tracing::info;

use super::state::ConnectedClient;
use crate::network::event::NetworkEvent;

pub async fn run_ping_reaper(
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
