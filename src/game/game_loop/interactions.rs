use std::sync::Arc;

use crate::game::component::interaction::{Direction, Movement};
use crate::game::player::Player;
use crate::game::{GameState, Interaction, Location, messaging};
use crate::persistence::Database;
use crate::persistence::{entity_repo, room_repo};

pub async fn process(game_state: &Arc<GameState>, db: &Database, tick: u64) {
    tracing::debug!("Processing interactions tick={tick}");

    let players: Vec<Player> = game_state
        .active_players
        .read()
        .await
        .values()
        .cloned()
        .collect();

    for player in players {
        let interactions = game_state.mailboxes.drain(player.entity_id).await;
        for interaction in interactions {
            match interaction {
                Interaction::Movement(Movement::TryDirection(direction)) => {
                    process_move(game_state, db, &player, direction).await;
                }
                Interaction::Movement(Movement::Warp(_)) => {}
            }
        }
    }
}

async fn process_move(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    direction: Direction,
) {
    let location = {
        let entities = game_state.active_entities.read().await;
        match entities.get(&player.entity_id) {
            Some(e) => e.location.clone(),
            None => return,
        }
    };

    let room = match room_repo::find_by_id(db.pool(), &location.room_id).await {
        Ok(Some(r)) => r,
        _ => return,
    };

    let nav = match direction {
        Direction::North => room.north,
        Direction::South => room.south,
        Direction::East => room.east,
        Direction::West => room.west,
    };

    match nav {
        Some(nav) => {
            let room_id = match nav.room_id {
                Some(id) => id,
                None => {
                    messaging::announce(
                        &game_state.message_tx,
                        player.id,
                        "Nothing in that direction.",
                    );
                    return;
                }
            };
            let new_location = Location {
                world_id: nav.world_id.unwrap_or(location.world_id),
                dungeon_id: nav.dungeon_id.unwrap_or(location.dungeon_id),
                room_id,
            };
            {
                let mut entities = game_state.active_entities.write().await;
                if let Some(entity) = entities.get_mut(&player.entity_id) {
                    entity.location = new_location.clone();
                }
            }
            if let Err(e) =
                entity_repo::update_location(db.pool(), player.entity_id, &new_location).await
            {
                tracing::error!(error = %e, "Failed to update entity location in DB");
            }
            messaging::announce(
                &game_state.message_tx,
                player.id,
                format!("You move {direction}."),
            );
        }
        None => {
            messaging::announce(
                &game_state.message_tx,
                player.id,
                "Nothing in that direction.",
            );
        }
    }
}
