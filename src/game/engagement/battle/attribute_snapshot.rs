use std::collections::HashMap;
use std::sync::Arc;

use crate::game::GameState;
use crate::game::component::Attribute;
use crate::game::component::attribute_definition::ResetCondition;
use crate::game::config::AttributeConfig;
use crate::game::entity::character::Character;

/// Per-character attribute values as of some snapshot point, keyed by character id.
pub type AttributeSnapshot = HashMap<i64, HashMap<String, Attribute>>;

/// Per-engagement attribute snapshots used to restore live attribute values on a schedule driven
/// by each `AttributeDefinition`'s `ResetCondition`. `battle_start` is captured once when the
/// battle is created and never changes; `turn_start` is a working record recaptured at the start
/// of every faction turn — it is not itself a reset source.
#[derive(Debug, Clone, Default)]
pub struct AttributeSnapshots {
    pub battle_start: AttributeSnapshot,
    pub turn_start: AttributeSnapshot,
}

/// Captures the battle-start attribute snapshot for a freshly created battle: the live attribute
/// values of every participant, taken once. Used later as the restore target for both
/// `EachEngagementTurn` and `EndOfEngagement` resets. Call this exactly once, immediately after
/// the engagement has been inserted into `Battles`.
pub async fn capture_battle_start(
    game_state: &Arc<GameState>,
    engagement_id: i64,
    entity_ids: &[i64],
) {
    let snapshot = read_attribute_snapshot(game_state, entity_ids).await;
    game_state
        .engagements
        .battles
        .set_battle_start_snapshot(engagement_id, snapshot)
        .await;
}

/// Runs the `ResetAttributes` phase's attribute work for one faction turn: resets every
/// `EachEngagementTurn` attribute on every given character back to its `battle_start` value, then
/// recaptures `turn_start` from the (now-reset) live values. Only call this when the current
/// battle tick has just completed a `ResetAttributes` phase.
pub(super) async fn reset_turn_start_attributes(
    game_state: &Arc<GameState>,
    engagement_id: i64,
    entity_ids: &[i64],
    config: &AttributeConfig,
) {
    let Some(battle_start) = game_state
        .engagements
        .battles
        .battle_start_snapshot(engagement_id)
        .await
    else {
        return;
    };
    let reset_ids = reset_condition_ids(config, &ResetCondition::EachEngagementTurn);

    let turn_start = {
        let mut entities = game_state.active_characters.write().await;
        reset_attributes(&mut entities, entity_ids, &battle_start, &reset_ids);
        snapshot_locked(&entities, entity_ids)
    };

    log_reset_attributes(engagement_id, entity_ids, &turn_start, &reset_ids);

    game_state
        .engagements
        .battles
        .set_turn_start_snapshot(engagement_id, turn_start)
        .await;
}

fn log_reset_attributes(
    engagement_id: i64,
    entity_ids: &[i64],
    turn_start: &AttributeSnapshot,
    reset_ids: &[&str],
) {
    for &entity_id in entity_ids {
        let Some(attrs) = turn_start.get(&entity_id) else {
            continue;
        };
        let pairs = attribute_pairs(attrs, reset_ids);
        tracing::info!(
            engagement_id,
            entity_id,
            pending_attributes = ?pairs,
            "Pending attributes reset for character"
        );
    }
}

fn attribute_pairs(attrs: &HashMap<String, Attribute>, ids: &[&str]) -> Vec<(String, i64)> {
    let mut pairs: Vec<(String, i64)> = ids
        .iter()
        .filter_map(|&id| attrs.get(id).map(|a| (id.to_string(), a.current_value)))
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs
}

/// Resets every `EndOfEngagement` attribute on every given character back to its `battle_start`
/// value. Only call this when a battle has just reached `BattlePhase::Concluded`, before the
/// engagement is removed from `Battles`.
pub(super) async fn reset_end_of_engagement_attributes(
    game_state: &Arc<GameState>,
    engagement_id: i64,
    entity_ids: &[i64],
    config: &AttributeConfig,
) {
    let Some(battle_start) = game_state
        .engagements
        .battles
        .battle_start_snapshot(engagement_id)
        .await
    else {
        return;
    };
    let reset_ids = reset_condition_ids(config, &ResetCondition::EndOfEngagement);

    let mut entities = game_state.active_characters.write().await;
    reset_attributes(&mut entities, entity_ids, &battle_start, &reset_ids);
}

async fn read_attribute_snapshot(
    game_state: &Arc<GameState>,
    entity_ids: &[i64],
) -> AttributeSnapshot {
    let entities = game_state.active_characters.read().await;
    snapshot_locked(&entities, entity_ids)
}

fn snapshot_locked(entities: &HashMap<i64, Character>, entity_ids: &[i64]) -> AttributeSnapshot {
    entity_ids
        .iter()
        .filter_map(|&id| entities.get(&id).map(|e| (id, e.attributes.clone())))
        .collect()
}

fn reset_condition_ids<'a>(
    config: &'a AttributeConfig,
    condition: &ResetCondition,
) -> Vec<&'a str> {
    config
        .attributes
        .iter()
        .filter(|def| &def.reset_condition == condition)
        .map(|def| def.id.as_str())
        .collect()
}

fn reset_attributes(
    entities: &mut HashMap<i64, Character>,
    entity_ids: &[i64],
    battle_start: &AttributeSnapshot,
    reset_ids: &[&str],
) {
    for &id in entity_ids {
        let Some(character) = entities.get_mut(&id) else {
            continue;
        };
        let Some(start_attrs) = battle_start.get(&id) else {
            continue;
        };
        for &attr_id in reset_ids {
            if let (Some(current), Some(start)) = (
                character.attributes.get_mut(attr_id),
                start_attrs.get(attr_id),
            ) {
                *current = start.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::Location;
    use crate::game::entity::character::CharacterType;

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    fn entity_with_attrs(id: i64, hp: i64, strength: i64) -> Character {
        let mut character = Character::new(id, CharacterType::Player, test_location());
        character.attributes.insert(
            "hp".to_string(),
            Attribute::new("hp".to_string(), 0, 100, hp),
        );
        character.attributes.insert(
            "strength".to_string(),
            Attribute::new("strength".to_string(), 1, 20, strength),
        );
        character
    }

    /// `AttributeConfig::default_config` has no `EndOfEngagement` attributes (only `Never` and
    /// `EachEngagementTurn`), so tests exercising the end-of-engagement reset re-tag "strength".
    fn config_with_end_of_engagement_strength() -> AttributeConfig {
        let mut config = AttributeConfig::default_config();
        if let Some(def) = config.attributes.iter_mut().find(|d| d.id == "strength") {
            def.reset_condition = ResetCondition::EndOfEngagement;
        }
        config
    }

    async fn game_state_with_battle(character: Character) -> (Arc<GameState>, i64) {
        let entity_id = character.id;
        let game_state = Arc::new(GameState::load(None).unwrap());
        game_state
            .active_characters
            .write()
            .await
            .insert(entity_id, character);
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![entity_id]);
        let engagement_id = game_state
            .engagements
            .add_battle("room".to_string(), vec!["player".to_string()], participants)
            .await;
        (game_state, engagement_id)
    }

    #[tokio::test]
    async fn capture_battle_start_stores_snapshot_once_and_is_stable() {
        let (game_state, engagement_id) =
            game_state_with_battle(entity_with_attrs(1, 100, 10)).await;

        capture_battle_start(&game_state, engagement_id, &[1]).await;

        game_state
            .active_characters
            .write()
            .await
            .get_mut(&1)
            .unwrap()
            .attributes
            .get_mut("strength")
            .unwrap()
            .current_value = 3;

        let snapshot = game_state
            .engagements
            .battles
            .battle_start_snapshot(engagement_id)
            .await
            .unwrap();
        assert_eq!(snapshot[&1]["strength"].current_value, 10);
    }

    #[tokio::test]
    async fn reset_turn_start_attributes_resets_each_engagement_turn_and_leaves_never() {
        let (game_state, engagement_id) =
            game_state_with_battle(entity_with_attrs(1, 100, 10)).await;
        capture_battle_start(&game_state, engagement_id, &[1]).await;

        {
            let mut entities = game_state.active_characters.write().await;
            let character = entities.get_mut(&1).unwrap();
            character
                .attributes
                .get_mut("strength")
                .unwrap()
                .current_value = 3;
            character.attributes.get_mut("hp").unwrap().current_value = 40;
        }

        reset_turn_start_attributes(
            &game_state,
            engagement_id,
            &[1],
            &AttributeConfig::default_config(),
        )
        .await;

        let entities = game_state.active_characters.read().await;
        assert_eq!(entities[&1].attributes["strength"].current_value, 10);
        assert_eq!(entities[&1].attributes["hp"].current_value, 40);
    }

    #[tokio::test]
    async fn reset_turn_start_attributes_recaptures_turn_start_from_post_reset_values() {
        let (game_state, engagement_id) =
            game_state_with_battle(entity_with_attrs(1, 100, 10)).await;
        capture_battle_start(&game_state, engagement_id, &[1]).await;
        game_state
            .active_characters
            .write()
            .await
            .get_mut(&1)
            .unwrap()
            .attributes
            .get_mut("strength")
            .unwrap()
            .current_value = 3;

        reset_turn_start_attributes(
            &game_state,
            engagement_id,
            &[1],
            &AttributeConfig::default_config(),
        )
        .await;

        let map = game_state.engagements.battles.map.read().await;
        let turn_start = &map[&engagement_id].attribute_snapshots.turn_start;
        assert_eq!(turn_start[&1]["strength"].current_value, 10);
    }

    #[tokio::test]
    async fn reset_end_of_engagement_attributes_resets_only_end_of_engagement_and_leaves_never() {
        let (game_state, engagement_id) =
            game_state_with_battle(entity_with_attrs(1, 100, 10)).await;
        capture_battle_start(&game_state, engagement_id, &[1]).await;

        {
            let mut entities = game_state.active_characters.write().await;
            let character = entities.get_mut(&1).unwrap();
            character
                .attributes
                .get_mut("strength")
                .unwrap()
                .current_value = 3;
            character.attributes.get_mut("hp").unwrap().current_value = 40;
        }

        reset_end_of_engagement_attributes(
            &game_state,
            engagement_id,
            &[1],
            &config_with_end_of_engagement_strength(),
        )
        .await;

        let entities = game_state.active_characters.read().await;
        assert_eq!(entities[&1].attributes["strength"].current_value, 10);
        assert_eq!(entities[&1].attributes["hp"].current_value, 40);
    }

    #[tokio::test]
    async fn reset_is_no_op_when_battle_start_snapshot_absent() {
        let (game_state, engagement_id) =
            game_state_with_battle(entity_with_attrs(1, 100, 10)).await;
        // Note: capture_battle_start is intentionally never called here.

        reset_turn_start_attributes(
            &game_state,
            engagement_id,
            &[1],
            &AttributeConfig::default_config(),
        )
        .await;

        let entities = game_state.active_characters.read().await;
        assert_eq!(entities[&1].attributes["strength"].current_value, 10);
    }
}
