use std::collections::HashMap;
use std::sync::Arc;

use crate::game::GameState;
use crate::game::component::OnZeroTrigger;
use crate::game::config::AttributeConfig;
use crate::game::entity::character::Character;

/// Detects which of the given entities have died (an attribute whose `on_zero` trigger is
/// `Death` has hit its minimum value).
/// Only call this when the current battle tick has just completed the `ResolveEntityState` phase.
pub(super) async fn detect_dead_entities(
    game_state: &Arc<GameState>,
    entity_ids: &[i64],
    config: &AttributeConfig,
) -> Vec<i64> {
    let death_trigger_def_ids: Vec<&str> = config
        .attributes
        .iter()
        .filter(|def| def.on_zero == OnZeroTrigger::Death)
        .map(|def| def.id.as_str())
        .collect();

    let entities = game_state.active_characters.read().await;
    entity_ids
        .iter()
        .filter(|&&id| is_entity_dead(id, &entities, &death_trigger_def_ids))
        .copied()
        .collect()
}

fn is_entity_dead(
    entity_id: i64,
    entities: &HashMap<i64, Character>,
    death_trigger_def_ids: &[&str],
) -> bool {
    let Some(character) = entities.get(&entity_id) else {
        return false;
    };
    death_trigger_def_ids.iter().any(|&def_id| {
        character
            .effective_attribute(def_id)
            .is_some_and(|attr| attr.current_value <= attr.min_value)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::Attribute;
    use crate::game::component::Location;
    use crate::game::entity::character::CharacterType;

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    fn entity_with_hp(id: i64, current: i64) -> Character {
        let mut character = Character::new(id, CharacterType::Player, test_location());
        character.attributes.insert(
            "hp".to_string(),
            Attribute::new("hp".to_string(), 0, 100, current),
        );
        character
    }

    async fn game_state_with_entities(entities: Vec<Character>) -> Arc<GameState> {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let mut map = game_state.active_characters.write().await;
        for character in entities {
            map.insert(character.id, character);
        }
        drop(map);
        game_state
    }

    #[tokio::test]
    async fn detect_dead_entities_returns_ids_at_or_below_min_hp() {
        let game_state =
            game_state_with_entities(vec![entity_with_hp(1, 0), entity_with_hp(2, 50)]).await;

        let dead =
            detect_dead_entities(&game_state, &[1, 2], &AttributeConfig::default_config()).await;

        assert_eq!(dead, vec![1]);
    }

    #[tokio::test]
    async fn detect_dead_entities_returns_empty_when_all_alive() {
        let game_state =
            game_state_with_entities(vec![entity_with_hp(1, 10), entity_with_hp(2, 50)]).await;

        let dead =
            detect_dead_entities(&game_state, &[1, 2], &AttributeConfig::default_config()).await;

        assert!(dead.is_empty());
    }

    #[tokio::test]
    async fn detect_dead_entities_ignores_missing_entities() {
        let game_state = game_state_with_entities(vec![]).await;

        let dead =
            detect_dead_entities(&game_state, &[99], &AttributeConfig::default_config()).await;

        assert!(dead.is_empty());
    }

    #[tokio::test]
    async fn detect_dead_entities_ignores_non_death_trigger_attribute_at_min() {
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character.attributes.insert(
            "mp".to_string(),
            Attribute::new("mp".to_string(), 0, 100, 0),
        );
        let game_state = game_state_with_entities(vec![character]).await;

        let dead =
            detect_dead_entities(&game_state, &[1], &AttributeConfig::default_config()).await;

        assert!(dead.is_empty());
    }

    #[test]
    fn is_entity_dead_true_when_current_at_min() {
        let mut entities = HashMap::new();
        entities.insert(1, entity_with_hp(1, 0));
        assert!(is_entity_dead(1, &entities, &["hp"]));
    }

    #[test]
    fn is_entity_dead_false_when_above_min() {
        let mut entities = HashMap::new();
        entities.insert(1, entity_with_hp(1, 1));
        assert!(!is_entity_dead(1, &entities, &["hp"]));
    }

    #[test]
    fn is_entity_dead_false_for_unknown_entity() {
        let entities = HashMap::new();
        assert!(!is_entity_dead(1, &entities, &["hp"]));
    }

    fn buff_effect(attribute_id: &str, value: i64) -> crate::game::component::effect::Effect {
        use crate::game::component::effect::{
            EffectDescription, EffectScope, EffectType, TriggerInfo,
        };
        crate::game::component::effect::Effect {
            name: "buff".to_string(),
            effect_type: EffectType::AttributeBuff {
                attribute_id: attribute_id.to_string(),
                value,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
            scope: EffectScope::default(),
        }
    }

    #[test]
    fn is_entity_dead_false_when_a_buff_keeps_effective_hp_above_zero() {
        let mut entities = HashMap::new();
        let mut character = entity_with_hp(1, 0);
        character.active_effects.push(buff_effect("hp", 10));
        entities.insert(1, character);
        assert!(!is_entity_dead(1, &entities, &["hp"]));
    }

    #[test]
    fn is_entity_dead_true_when_a_debuff_pushes_effective_hp_to_zero() {
        let mut entities = HashMap::new();
        let mut character = entity_with_hp(1, 5);
        character.active_effects.push(buff_effect("hp", -5));
        entities.insert(1, character);
        assert!(is_entity_dead(1, &entities, &["hp"]));
    }
}
