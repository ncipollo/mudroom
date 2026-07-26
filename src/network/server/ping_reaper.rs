use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;
use tracing::info;

use super::state::ConnectedClient;
use crate::game::GameState;

pub async fn run_ping_reaper(
    connections: Arc<RwLock<HashMap<String, ConnectedClient>>>,
    game_state: Arc<GameState>,
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
                let entity_id = game_state
                    .active_players
                    .read()
                    .await
                    .get(&id)
                    .map(|p| p.entity_id);
                info!(
                    client_id = %id,
                    entity_id,
                    "player disconnected due to ping/pong timeout"
                );
            }
        }
    }
}
