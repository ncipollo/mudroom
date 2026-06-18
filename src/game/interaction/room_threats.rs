use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tracing;

use crate::game::component::faction_relations::FactionRelation;
use crate::game::entity::Entity;
use crate::game::player::Player;
use crate::game::{GameState, messaging};

enum RoomThreat {
    Hostile,
    Unfriendly,
    None,
}

pub async fn check_room_hostility(game_state: &Arc<GameState>, player: &Player, room_id: &str) {
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
    let (factions, participants) =
        build_participants(game_state, player.entity_id, &hostile_ids).await;
    let all_ids: Vec<i64> = participants.values().flatten().copied().collect();
    tracing::info!(
        room_id,
        entity_ids = ?all_ids,
        "battle started"
    );
    game_state
        .engagements
        .add_battle(room_id.to_string(), factions, participants)
        .await;
    messaging::message(
        &game_state.message_tx,
        player.id,
        "Hostile entities attack! A battle has started.",
    );
}

async fn build_participants(
    game_state: &Arc<GameState>,
    player_entity_id: i64,
    hostile_ids: &[i64],
) -> (Vec<String>, HashMap<String, Vec<i64>>) {
    let entities = game_state.active_entities.read().await;

    let mut participants: HashMap<String, Vec<i64>> = HashMap::new();
    for &entity_id in std::iter::once(&player_entity_id).chain(hostile_ids.iter()) {
        if let Some(entity) = entities.get(&entity_id) {
            for faction in &entity.factions {
                participants
                    .entry(faction.clone())
                    .or_default()
                    .push(entity_id);
            }
        }
    }

    let factions = ordered_factions(&entities, player_entity_id, &participants);
    (factions, participants)
}

fn ordered_factions(
    entities: &HashMap<i64, Entity>,
    player_entity_id: i64,
    participants: &HashMap<String, Vec<i64>>,
) -> Vec<String> {
    let mut factions: Vec<String> = Vec::new();
    if let Some(player_entity) = entities.get(&player_entity_id) {
        for faction in &player_entity.factions {
            factions.push(faction.clone());
        }
    }
    for faction in participants.keys() {
        if !factions.contains(faction) {
            factions.push(faction.clone());
        }
    }
    factions
}

fn entity_threat_toward_player(
    entity_faction_relations: &HashMap<String, FactionRelation>,
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
