use std::collections::HashMap;
use std::sync::Arc;

use crate::game::GameState;
use crate::game::component::AttributeType;
use crate::game::config::AttributeConfig;
use crate::game::entity::Character;

/// Detects which of the given entities have died (an HP-type attribute at or below its minimum).
/// Only call this when the current battle tick has just completed the `ResolveEntityState` phase.
pub(super) async fn detect_dead_entities(
    game_state: &Arc<GameState>,
    entity_ids: &[i64],
    config: &AttributeConfig,
) -> Vec<i64> {
    let hp_def_ids: Vec<&str> = config
        .attributes
        .iter()
        .filter(|def| matches!(def.attribute_type, AttributeType::HP))
        .map(|def| def.id.as_str())
        .collect();

    let entities = game_state.active_characters.read().await;
    entity_ids
        .iter()
        .filter(|&&id| is_entity_dead(id, &entities, &hp_def_ids))
        .copied()
        .collect()
}

fn is_entity_dead(entity_id: i64, entities: &HashMap<i64, Character>, hp_def_ids: &[&str]) -> bool {
    let Some(character) = entities.get(&entity_id) else {
        return false;
    };
    hp_def_ids.iter().any(|&hp_id| {
        character
            .attributes
            .get(hp_id)
            .is_some_and(|attr| attr.current_value <= attr.min_value)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::Attribute;
    use crate::game::component::Location;
    use crate::game::entity::CharacterType;

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
}
