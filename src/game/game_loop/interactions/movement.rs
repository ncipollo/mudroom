use std::sync::Arc;

use crate::game::component::interaction::Direction;
use crate::game::player::Player;
use crate::game::{GameState, Location, messaging};
use crate::persistence::Database;
use crate::persistence::{entity_repo, room_repo};

pub async fn process(
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

    let room = match room_repo::find_by_id(db.pool(), &location.dungeon_id, &location.room_id).await
    {
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
                    messaging::message(
                        &game_state.message_tx,
                        player.id,
                        "Nothing in that direction.",
                    );
                    return;
                }
            };
            let old_world_id = location.world_id.clone();
            let old_dungeon_id = location.dungeon_id.clone();
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
            if (new_location.world_id != old_world_id || new_location.dungeon_id != old_dungeon_id)
                && let Err(e) = game_state.sync_active_entities(db.pool()).await
            {
                tracing::error!(error = %e, "Failed to sync active entities after dungeon change");
            }
            messaging::message(
                &game_state.message_tx,
                player.id,
                format!("You move {direction}."),
            );
            if let Ok(Some(new_room)) =
                room_repo::find_by_id(db.pool(), &new_location.dungeon_id, &new_location.room_id)
                    .await
            {
                messaging::message_room_description(&game_state.message_tx, player.id, &new_room);
            }
        }
        None => {
            messaging::message(
                &game_state.message_tx,
                player.id,
                "Nothing in that direction.",
            );
        }
    }
}
