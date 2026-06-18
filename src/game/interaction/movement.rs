use std::collections::HashSet;
use std::sync::Arc;

use tracing;

use crate::game::component::faction_relations::FactionRelation;
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
    check_room_hostility(game_state, player, &new_location.room_id).await;
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

enum RoomThreat {
    Hostile,
    Unfriendly,
    None,
}

async fn check_room_hostility(game_state: &Arc<GameState>, player: &Player, room_id: &str) {
    if game_state
        .engagements
        .find_battle_for_room(room_id)
        .await
        .is_some()
    {
        messaging::message(
            &game_state.message_tx,
            player.id,
            "A battle is already underway here! You can join or flee.",
        );
        return;
    }

    let (hostile_ids, unfriendly_ids) =
        scan_room_threats(game_state, player.entity_id, room_id).await;

    if !hostile_ids.is_empty() {
        start_battle(game_state, player, room_id, hostile_ids).await;
    } else if !unfriendly_ids.is_empty() {
        messaging::message(
            &game_state.message_tx,
            player.id,
            "Some here look unfriendly. You could choose to engage them.",
        );
    }
}

async fn scan_room_threats(
    game_state: &Arc<GameState>,
    player_entity_id: i64,
    room_id: &str,
) -> (Vec<i64>, Vec<i64>) {
    let entities = game_state.active_entities.read().await;
    let player_factions = entities
        .get(&player_entity_id)
        .map(|e| e.factions.clone())
        .unwrap_or_default();

    let mut hostile_ids = Vec::new();
    let mut unfriendly_ids = Vec::new();
    for entity in entities.values() {
        if entity.id == player_entity_id || entity.location.room_id != room_id {
            continue;
        }
        match entity_threat_toward_player(&entity.faction_relations.factions, &player_factions) {
            RoomThreat::Hostile => hostile_ids.push(entity.id),
            RoomThreat::Unfriendly => unfriendly_ids.push(entity.id),
            RoomThreat::None => {}
        }
    }
    (hostile_ids, unfriendly_ids)
}

async fn start_battle(
    game_state: &Arc<GameState>,
    player: &Player,
    room_id: &str,
    hostile_ids: Vec<i64>,
) {
    let mut battle_ids = vec![player.entity_id];
    battle_ids.extend(hostile_ids);
    game_state
        .engagements
        .add_battle(room_id.to_string(), battle_ids)
        .await;
    messaging::message(
        &game_state.message_tx,
        player.id,
        "Hostile entities attack! A battle has started.",
    );
}

fn entity_threat_toward_player(
    entity_faction_relations: &std::collections::HashMap<String, FactionRelation>,
    player_factions: &HashSet<String>,
) -> RoomThreat {
    let mut found_unfriendly = false;
    for faction_id in player_factions {
        match entity_faction_relations
            .get(faction_id)
            .unwrap_or(&FactionRelation::NonInteractive)
        {
            FactionRelation::Hostile => return RoomThreat::Hostile,
            FactionRelation::Unfriendly => found_unfriendly = true,
            _ => {}
        }
    }
    if found_unfriendly {
        RoomThreat::Unfriendly
    } else {
        RoomThreat::None
    }
}
