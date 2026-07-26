use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;
use tracing::info;

use super::state::{ConnectedClient, queue_player_disconnected};
use crate::game::GameState;

pub async fn run_ping_reaper(
    connections: Arc<RwLock<HashMap<String, ConnectedClient>>>,
    game_state: Arc<GameState>,
) {
    let timeout = std::time::Duration::from_secs(30);
    let interval = std::time::Duration::from_secs(10);
    loop {
        tokio::time::sleep(interval).await;
        reap_stale_connections(&connections, &game_state, timeout).await;
    }
}

async fn reap_stale_connections(
    connections: &Arc<RwLock<HashMap<String, ConnectedClient>>>,
    game_state: &Arc<GameState>,
    timeout: std::time::Duration,
) {
    let now = Instant::now();
    let stale: Vec<String> = connections
        .read()
        .await
        .iter()
        .filter(|(_, c)| now.duration_since(c.last_ping) > timeout)
        .map(|(id, _)| id.clone())
        .collect();
    if stale.is_empty() {
        return;
    }
    let mut guard = connections.write().await;
    for id in &stale {
        guard.remove(id);
    }
    drop(guard);
    for id in stale {
        let entity_id = queue_player_disconnected(game_state, &id).await;
        info!(
            client_id = %id,
            entity_id,
            "player disconnected due to ping/pong timeout"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::Location;
    use crate::game::entity::{Entity, EntityType};
    use crate::game::interaction;
    use crate::game::player::Player;
    use crate::network::event::NetworkEvent;
    use crate::persistence::Database;
    use std::time::Duration;
    use tokio::sync::mpsc;

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    async fn battle_game_state(client_id: &str) -> Arc<GameState> {
        let game_state = Arc::new(GameState::load(None).unwrap());
        {
            let mut entities = game_state.active_entities.write().await;
            entities.insert(1, Entity::new(1, EntityType::Player, test_location()));
            entities.insert(2, Entity::new(2, EntityType::Enemy, test_location()));
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
        game_state
    }

    #[tokio::test]
    async fn reap_stale_connections_concludes_battle_for_timed_out_player() {
        let game_state = battle_game_state("m:1").await;
        let (personal_tx, _rx) = mpsc::channel::<NetworkEvent>(1);
        let connections = Arc::new(RwLock::new(HashMap::from([(
            "m:1".to_string(),
            ConnectedClient {
                last_ping: Instant::now() - Duration::from_secs(40),
                personal_tx,
                seq: 1,
            },
        )])));
        let timeout = Duration::from_secs(30);

        reap_stale_connections(&connections, &game_state, timeout).await;

        assert!(
            !connections.read().await.contains_key("m:1"),
            "timed-out connection should be reaped"
        );

        // Reaping only queues the disconnect; the game loop's interaction processing is what
        // actually tears the battle down.
        let db = Database::connect_in_memory().await.unwrap();
        interaction::process(&game_state, &db, 0).await;

        assert_eq!(
            game_state.engagements.battles.find_for_entity(2).await,
            None,
            "battle should conclude once the only player-faction member times out"
        );
    }

    #[tokio::test]
    async fn reap_stale_connections_leaves_fresh_connections_untouched() {
        let game_state = battle_game_state("m:2").await;
        let (personal_tx, _rx) = mpsc::channel::<NetworkEvent>(1);
        let connections = Arc::new(RwLock::new(HashMap::from([(
            "m:2".to_string(),
            ConnectedClient {
                last_ping: Instant::now(),
                personal_tx,
                seq: 1,
            },
        )])));
        let timeout = Duration::from_secs(30);

        reap_stale_connections(&connections, &game_state, timeout).await;

        assert!(connections.read().await.contains_key("m:2"));
        assert!(game_state.mailboxes.drain(1).await.is_empty());
    }
}
