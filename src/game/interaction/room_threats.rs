use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tracing;

use crate::game::component::attribute_definition::AttributeType;
use crate::game::component::faction_relations::FactionRelation;
use crate::game::engagement::battle::{
    BattlePhase, default_attack_ability, default_defend_ability, entity_innate_battle_abilities,
};
use crate::game::entity::Entity;
use crate::game::messaging::{BattleParticipantInfo, BattleStartedMessage};
use crate::game::player::Player;
use crate::game::{GameState, messaging};

mod participants;

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
        participants::build_participants(game_state, player.entity_id, &hostile_ids).await;
    let all_ids: Vec<i64> = participants.values().flatten().copied().collect();
    tracing::info!(
        room_id,
        entity_ids = ?all_ids,
        "battle started"
    );

    let engagement_id = game_state
        .engagements
        .add_battle(room_id.to_string(), factions.clone(), participants.clone())
        .await;

    let max_engage_ticks = (game_state.mud_config.game_loop.max_engage_ms
        / game_state.mud_config.game_loop.tick_rate_ms)
        .max(1);

    let started_msg = build_battle_started_message(
        game_state,
        player,
        engagement_id,
        &factions,
        &participants,
        max_engage_ticks,
    )
    .await;

    messaging::battle_started(&game_state.message_tx, player.id, started_msg);
    messaging::message(
        &game_state.message_tx,
        player.id,
        "Hostile entities attack! A battle has started.",
    );
}

async fn build_battle_started_message(
    game_state: &Arc<GameState>,
    player: &Player,
    engagement_id: i64,
    factions: &[String],
    participants: &HashMap<String, Vec<i64>>,
    max_turn_ticks: u64,
) -> BattleStartedMessage {
    let entities = game_state.active_entities.read().await;
    let players = game_state.active_players.read().await;
    let hp_attr_id = hp_attribute_id(&game_state.attribute_config);

    let participant_infos = build_participant_infos(participants, &entities, &players, &hp_attr_id);

    let mut available_abilities = entities
        .get(&player.entity_id)
        .map(entity_innate_battle_abilities)
        .unwrap_or_default();
    if available_abilities.is_empty() {
        available_abilities.push(default_attack_ability());
        available_abilities.push(default_defend_ability());
    }

    let mut turn_order: Vec<i64> = participants.values().flatten().copied().collect();
    turn_order.sort_unstable();

    let tick_rate_ms = game_state.mud_config.game_loop.tick_rate_ms;
    let max_turn_secs =
        crate::game::engagement::battle::timer::ticks_to_secs(max_turn_ticks, tick_rate_ms);

    BattleStartedMessage {
        engagement_id,
        factions: factions.to_vec(),
        participants: participant_infos,
        phase: BattlePhase::InnateEffects,
        turn_order,
        countdown_secs: max_turn_secs,
        max_turn_secs,
        available_abilities,
    }
}

fn build_participant_infos(
    participants: &HashMap<String, Vec<i64>>,
    entities: &HashMap<i64, Entity>,
    players: &HashMap<String, Player>,
    hp_attr_id: &str,
) -> HashMap<String, Vec<BattleParticipantInfo>> {
    participants
        .iter()
        .map(|(faction, ids)| {
            let infos = ids
                .iter()
                .map(|&id| {
                    let entity = entities.get(&id);
                    let name = resolve_entity_name(id, entity, players);
                    let (hp_current, hp_max) = entity
                        .and_then(|e| e.attributes.get(hp_attr_id))
                        .map(|a| (a.current_value, a.max_value))
                        .unwrap_or((0, 0));
                    BattleParticipantInfo {
                        id,
                        name,
                        hp_current,
                        hp_max,
                    }
                })
                .collect();
            (faction.clone(), infos)
        })
        .collect()
}

fn resolve_entity_name(
    entity_id: i64,
    entity: Option<&Entity>,
    players: &HashMap<String, Player>,
) -> String {
    if let Some(player) = players.values().find(|p| p.entity_id == entity_id) {
        return player.name.clone();
    }
    entity
        .and_then(|e| e.config_id.as_deref())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Entity {entity_id}"))
}

fn hp_attribute_id(attribute_config: &crate::game::config::AttributeConfig) -> String {
    attribute_config
        .attributes
        .iter()
        .find(|def| matches!(def.attribute_type, AttributeType::HP))
        .map(|def| def.id.clone())
        .unwrap_or_else(|| "hp".to_string())
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
