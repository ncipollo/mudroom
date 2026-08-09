use std::collections::HashMap;
use std::sync::Arc;

use crate::game::GameState;
use crate::game::engagement::battle::{BattleMessage, BattlePhase, entity_innate_battle_abilities};
use crate::game::messaging::{self, BattleParticipantInfo, BattleUpdateMessage};

pub(in crate::game::engagement::battle) struct BattleUpdateParams {
    pub engagement_id: i64,
    pub factions: Vec<String>,
    pub participants: HashMap<String, Vec<i64>>,
    pub phase: BattlePhase,
    pub messages: Vec<BattleMessage>,
    pub countdown_secs: u64,
    pub max_turn_secs: u64,
}

pub(in crate::game::engagement::battle) async fn build_battle_update(
    game_state: &Arc<GameState>,
    params: BattleUpdateParams,
) -> BattleUpdateMessage {
    let entities = game_state.active_characters.read().await;
    let hp_attr_id = messaging::hp_attribute_id(&game_state.attribute_config);

    let participant_infos = params
        .participants
        .iter()
        .map(|(faction, ids)| {
            let infos = ids
                .iter()
                .map(|&id| {
                    let character = entities.get(&id);
                    let name = character
                        .map(|e| e.name.clone())
                        .unwrap_or_else(|| format!("Character {id}"));
                    let (hp_current, hp_max) = character
                        .and_then(|e| e.attributes.get(&hp_attr_id))
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
        .collect();

    BattleUpdateMessage {
        engagement_id: params.engagement_id,
        factions: params.factions,
        participants: participant_infos,
        phase: params.phase,
        messages: params.messages,
        countdown_secs: params.countdown_secs,
        max_turn_secs: params.max_turn_secs,
        available_abilities: vec![],
    }
}

/// Sends a player-customized copy of `update` (with `available_abilities` filled in for that
/// player's character) to every participant in `player_pairs`.
pub(in crate::game::engagement::battle) async fn broadcast_update(
    game_state: &Arc<GameState>,
    player_pairs: &[(i64, i64)],
    update: &BattleUpdateMessage,
) {
    let entities = game_state.active_characters.read().await;
    for &(pid, entity_id) in player_pairs {
        let available_abilities = entities
            .get(&entity_id)
            .map(entity_innate_battle_abilities)
            .unwrap_or_default();
        let mut player_update = update.clone();
        player_update.available_abilities = available_abilities;
        messaging::battle_update(&game_state.message_tx, pid, player_update);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::character::{Character, CharacterType};
    use crate::game::component::Location;

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    fn params(participants: HashMap<String, Vec<i64>>) -> BattleUpdateParams {
        BattleUpdateParams {
            engagement_id: 1,
            factions: vec!["player".to_string()],
            participants,
            phase: BattlePhase::ResolveAbilities,
            messages: vec![],
            countdown_secs: 5,
            max_turn_secs: 10,
        }
    }

    #[tokio::test]
    async fn build_battle_update_fills_in_known_participant_hp() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character.name = "Hero".to_string();
        character.attributes.insert(
            "hp".to_string(),
            crate::game::component::Attribute::new("hp".to_string(), 0, 100, 42),
        );
        game_state
            .active_characters
            .write()
            .await
            .insert(1, character);

        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);

        let update = build_battle_update(&game_state, params(participants)).await;

        let infos = update.participants.get("player").unwrap();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].name, "Hero");
        assert_eq!(infos[0].hp_current, 42);
        assert_eq!(infos[0].hp_max, 100);
    }

    #[tokio::test]
    async fn build_battle_update_defaults_missing_entity_to_placeholder_name_and_zero_hp() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![99]);

        let update = build_battle_update(&game_state, params(participants)).await;

        let infos = update.participants.get("player").unwrap();
        assert_eq!(infos[0].name, "Character 99");
        assert_eq!(infos[0].hp_current, 0);
        assert_eq!(infos[0].hp_max, 0);
    }

    #[tokio::test]
    async fn broadcast_update_sends_a_message_per_player() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let mut receiver = game_state.message_tx.subscribe();
        let update = build_battle_update(&game_state, params(HashMap::new())).await;

        broadcast_update(&game_state, &[(7, 1), (8, 2)], &update).await;

        assert!(receiver.try_recv().is_ok());
        assert!(receiver.try_recv().is_ok());
    }
}
