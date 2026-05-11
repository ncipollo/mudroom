use std::sync::Arc;

use tracing;

use crate::game::component::interaction::Direction;
use crate::game::map::universe::navigation::Navigation;
use crate::game::player::Player;

use crate::game::{GameState, Location, messaging};
use crate::persistence::Database;
use crate::persistence::{entity_repo, room_repo};

use super::look;

struct NavTarget {
    world_id: Option<String>,
    dungeon_id: Option<String>,
    room_id: String,
}

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

    let Some(Navigation {
        room_id: Some(room_id),
        world_id,
        dungeon_id,
    }) = nav
    else {
        messaging::message(
            &game_state.message_tx,
            player.id,
            "Nothing in that direction.",
        );
        return;
    };

    let target = NavTarget {
        world_id,
        dungeon_id,
        room_id,
    };
    execute_move(game_state, db, player, location, target, direction).await;
}

async fn execute_move(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    location: Location,
    target: NavTarget,
    direction: Direction,
) {
    let old_location = location.clone();
    let new_location = Location {
        world_id: target.world_id.unwrap_or(location.world_id),
        dungeon_id: target.dungeon_id.unwrap_or(location.dungeon_id),
        room_id: target.room_id,
    };
    update_entity_location(game_state, db, player.entity_id, &new_location).await;
    sync_if_dungeon_changed(game_state, db, &old_location, &new_location).await;
    messaging::message(
        &game_state.message_tx,
        player.id,
        format!("You move {direction}."),
    );
    look::process(game_state, db, player).await;
}

async fn update_entity_location(
    game_state: &Arc<GameState>,
    db: &Database,
    entity_id: i64,
    new_location: &Location,
) {
    {
        let mut entities = game_state.active_entities.write().await;
        if let Some(entity) = entities.get_mut(&entity_id) {
            entity.location = new_location.clone();
        }
    }
    if let Err(e) = entity_repo::update_location(db.pool(), entity_id, new_location).await {
        tracing::error!("Failed to update entity location in DB: {e}");
    }
}

async fn sync_if_dungeon_changed(
    game_state: &Arc<GameState>,
    db: &Database,
    old_location: &Location,
    new_location: &Location,
) {
    let dungeon_changed = new_location.world_id != old_location.world_id
        || new_location.dungeon_id != old_location.dungeon_id;
    if dungeon_changed && let Err(e) = game_state.sync_active_entities(db.pool()).await {
        tracing::error!("Failed to sync active entities after dungeon change: {e}");
    }
}
