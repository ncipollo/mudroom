use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tracing;

use crate::game::component::faction_relations::FactionRelation;
use crate::game::config::AttributeConfig;
use crate::game::engagement::battle::{BattlePhase, entity_battle_abilities};
use crate::game::entity::character::Character;
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
        .battles
        .find_for_room(room_id)
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
    let entities = game_state.active_characters.read().await;
    let player_factions = entities
        .get(&player_entity_id)
        .map(|e| e.factions.clone())
        .unwrap_or_default();

    let mut hostile_ids = Vec::new();
    let mut unfriendly_ids = Vec::new();
    for character in entities.values() {
        if character.id == player_entity_id || character.location.room_id != room_id {
            continue;
        }
        match entity_threat_toward_player(&character.faction_relations.factions, &player_factions) {
            RoomThreat::Hostile => hostile_ids.push(character.id),
            RoomThreat::Unfriendly => unfriendly_ids.push(character.id),
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

pub(crate) async fn build_battle_started_message(
    game_state: &Arc<GameState>,
    player: &Player,
    engagement_id: i64,
    factions: &[String],
    participants: &HashMap<String, Vec<i64>>,
    max_turn_ticks: u64,
) -> BattleStartedMessage {
    let entities = game_state.active_characters.read().await;
    let players = game_state.active_players.read().await;
    let item_definitions = game_state.item_definitions.read().await;
    let abilities = game_state.abilities.read().await;
    let hp_attr_id = messaging::hp_attribute_id(&game_state.attribute_config);

    let participant_infos = build_participant_infos(
        participants,
        &entities,
        &players,
        &hp_attr_id,
        &game_state.attribute_config,
    );

    let available_abilities = entities
        .get(&player.entity_id)
        .map(|c| entity_battle_abilities(c, &item_definitions, &abilities))
        .unwrap_or_default();

    let mut turn_order: Vec<i64> = participants.values().flatten().copied().collect();
    turn_order.sort_unstable();

    let tick_rate_ms = game_state.mud_config.game_loop.tick_rate_ms;
    let max_turn_secs =
        crate::game::engagement::battle::timer::ticks_to_secs(max_turn_ticks, tick_rate_ms);

    BattleStartedMessage {
        engagement_id,
        factions: factions.to_vec(),
        participants: participant_infos,
        phase: BattlePhase::ResetAttributes {
            faction: factions.first().cloned().unwrap_or_default(),
        },
        turn_order,
        countdown_secs: max_turn_secs,
        max_turn_secs,
        available_abilities,
    }
}

fn build_participant_infos(
    participants: &HashMap<String, Vec<i64>>,
    entities: &HashMap<i64, Character>,
    players: &HashMap<String, Player>,
    hp_attr_id: &str,
    attribute_config: &AttributeConfig,
) -> HashMap<String, Vec<BattleParticipantInfo>> {
    participants
        .iter()
        .map(|(faction, ids)| {
            let infos = ids
                .iter()
                .map(|&id| {
                    let character = entities.get(&id);
                    let name = resolve_entity_name(id, character, players);
                    let (hp_current, hp_max) = character
                        .and_then(|e| e.attributes.get(hp_attr_id))
                        .map(|a| (a.current_value, a.max_value))
                        .unwrap_or((0, 0));
                    BattleParticipantInfo {
                        id,
                        name,
                        hp_current,
                        hp_max,
                        attributes: messaging::participant_attributes(character, attribute_config),
                    }
                })
                .collect();
            (faction.clone(), infos)
        })
        .collect()
}

fn resolve_entity_name(
    entity_id: i64,
    character: Option<&Character>,
    players: &HashMap<String, Player>,
) -> String {
    if let Some(player) = players.values().find(|p| p.entity_id == entity_id) {
        return player.name.clone();
    }
    character
        .and_then(|e| e.config_id.as_deref())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Character {entity_id}"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::{Attribute, AttributeDefinition, Location, OnZeroTrigger};
    use crate::game::entity::character::CharacterType;

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    fn attribute_config_with(ids: &[&str]) -> AttributeConfig {
        AttributeConfig {
            attributes: ids
                .iter()
                .map(|id| AttributeDefinition {
                    id: id.to_string(),
                    title: id.to_string(),
                    description: String::new(),
                    min_value: 0,
                    max_value: 100,
                    on_zero: OnZeroTrigger::None,
                    updatable: true,
                })
                .collect(),
        }
    }

    #[test]
    fn build_participant_infos_includes_attributes_in_config_order() {
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character.attributes.insert(
            "hp".to_string(),
            Attribute::new("hp".to_string(), 0, 100, 42),
        );
        character.attributes.insert(
            "strength".to_string(),
            Attribute::new("strength".to_string(), 0, 20, 12),
        );
        let mut entities = HashMap::new();
        entities.insert(1, character);

        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);

        let config = attribute_config_with(&["hp", "strength"]);
        let infos =
            build_participant_infos(&participants, &entities, &HashMap::new(), "hp", &config);

        let attributes = &infos.get("player").unwrap()[0].attributes;
        assert_eq!(attributes.len(), 2);
        assert_eq!(attributes[0].id, "hp");
        assert_eq!(attributes[0].current, 42);
        assert_eq!(attributes[1].id, "strength");
        assert_eq!(attributes[1].current, 12);
    }

    #[test]
    fn build_participant_infos_empty_attributes_for_missing_entity() {
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![99]);

        let config = attribute_config_with(&["hp"]);
        let infos = build_participant_infos(
            &participants,
            &HashMap::new(),
            &HashMap::new(),
            "hp",
            &config,
        );

        assert!(infos.get("player").unwrap()[0].attributes.is_empty());
    }
}
