use crate::game::component::Attribute;
use crate::game::component::effect::EffectType;

use super::Character;

impl Character {
    /// The attribute's current value plus any active `AttributeBuff` effects targeting it,
    /// clamped to the attribute's min/max. Buffs never mutate `current_value` — they're a
    /// temporary overlay computed here, so callers that need the "real" temporary-adjusted value
    /// (death detection, turn order, status display) should read through this instead of
    /// `attributes` directly.
    pub fn effective_attribute(&self, attribute_id: &str) -> Option<Attribute> {
        let attribute = self.attributes.get(attribute_id)?;
        let mut effective = attribute.clone();
        effective.current_value = (attribute.current_value + self.buff_total(attribute_id))
            .clamp(attribute.min_value, attribute.max_value);
        Some(effective)
    }

    fn buff_total(&self, attribute_id: &str) -> i64 {
        self.active_effects
            .iter()
            .filter_map(|effect| match &effect.effect_type {
                EffectType::AttributeBuff {
                    attribute_id: buffed_id,
                    value,
                } if buffed_id == attribute_id => Some(*value),
                _ => None,
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use crate::game::component::Location;
    use crate::game::component::effect::{Effect, EffectDescription, EffectScope, TriggerInfo};
    use crate::game::entity::character::CharacterType;

    use super::*;

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    fn character_with_hp(current: i64, min: i64, max: i64) -> Character {
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character.attributes.insert(
            "hp".to_string(),
            Attribute::new("hp".to_string(), min, max, current),
        );
        character
    }

    fn buff(attribute_id: &str, value: i64) -> Effect {
        Effect {
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
    fn effective_attribute_matches_base_value_with_no_buffs() {
        let character = character_with_hp(50, 0, 100);
        assert_eq!(
            character.effective_attribute("hp").unwrap().current_value,
            50
        );
    }

    #[test]
    fn effective_attribute_adds_a_positive_buff() {
        let mut character = character_with_hp(50, 0, 100);
        character.active_effects.push(buff("hp", 20));
        assert_eq!(
            character.effective_attribute("hp").unwrap().current_value,
            70
        );
    }

    #[test]
    fn effective_attribute_clamps_to_max() {
        let mut character = character_with_hp(90, 0, 100);
        character.active_effects.push(buff("hp", 20));
        assert_eq!(
            character.effective_attribute("hp").unwrap().current_value,
            100
        );
    }

    #[test]
    fn effective_attribute_clamps_to_min_for_a_debuff() {
        let mut character = character_with_hp(10, 0, 100);
        character.active_effects.push(buff("hp", -20));
        assert_eq!(
            character.effective_attribute("hp").unwrap().current_value,
            0
        );
    }

    #[test]
    fn effective_attribute_sums_multiple_buffs() {
        let mut character = character_with_hp(50, 0, 100);
        character.active_effects.push(buff("hp", 10));
        character.active_effects.push(buff("hp", 5));
        assert_eq!(
            character.effective_attribute("hp").unwrap().current_value,
            65
        );
    }

    #[test]
    fn effective_attribute_ignores_buffs_for_other_attributes() {
        let mut character = character_with_hp(50, 0, 100);
        character.active_effects.push(buff("mp", 20));
        assert_eq!(
            character.effective_attribute("hp").unwrap().current_value,
            50
        );
    }

    #[test]
    fn effective_attribute_is_none_for_unknown_attribute() {
        let character = character_with_hp(50, 0, 100);
        assert!(character.effective_attribute("mp").is_none());
    }
}
