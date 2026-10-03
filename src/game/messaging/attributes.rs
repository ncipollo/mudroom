use crate::game::component::AttributeRenderInfo;
use crate::game::config::AttributeConfig;
use crate::game::entity::character::Character;

pub fn hp_attribute_id(attribute_config: &AttributeConfig) -> String {
    attribute_id_for(attribute_config, "hp")
}

pub fn mp_attribute_id(attribute_config: &AttributeConfig) -> String {
    attribute_id_for(attribute_config, "mp")
}

/// Falls back to `well_known_id` itself when undeclared, preserving the prior fail-open lookup.
fn attribute_id_for(attribute_config: &AttributeConfig, well_known_id: &str) -> String {
    attribute_config
        .attributes
        .iter()
        .find(|def| def.id == well_known_id)
        .map(|def| def.id.clone())
        .unwrap_or_else(|| well_known_id.to_string())
}

/// Full attribute list for a participant, in `AttributeConfig` order, read through
/// `Character::effective_attribute` so buffs active for the current phase are included.
pub fn participant_attributes(
    character: Option<&Character>,
    attribute_config: &AttributeConfig,
) -> Vec<AttributeRenderInfo> {
    let Some(character) = character else {
        return Vec::new();
    };
    attribute_config
        .attributes
        .iter()
        .filter_map(|def| {
            character
                .effective_attribute(&def.id)
                .map(|attr| AttributeRenderInfo {
                    id: def.id.clone(),
                    title: def.title.clone(),
                    current: attr.current_value,
                    max: attr.max_value,
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::effect::{
        Effect, EffectDescription, EffectScope, EffectType, TriggerInfo,
    };
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

    fn character_with_attributes(entries: &[(&str, i64, i64)]) -> Character {
        let mut character = Character::new(1, CharacterType::Player, test_location());
        for (id, current, max) in entries {
            character.attributes.insert(
                (*id).to_string(),
                Attribute::new((*id).to_string(), 0, *max, *current),
            );
        }
        character
    }

    #[test]
    fn participant_attributes_follows_config_order_and_skips_missing() {
        let config = attribute_config_with(&["hp", "mp", "strength"]);
        let character = character_with_attributes(&[("strength", 12, 20), ("hp", 42, 100)]);

        let attributes = participant_attributes(Some(&character), &config);

        assert_eq!(attributes.len(), 2);
        assert_eq!(attributes[0].id, "hp");
        assert_eq!(attributes[0].current, 42);
        assert_eq!(attributes[0].max, 100);
        assert_eq!(attributes[1].id, "strength");
        assert_eq!(attributes[1].current, 12);
        assert_eq!(attributes[1].max, 20);
    }

    #[test]
    fn participant_attributes_returns_empty_for_missing_character() {
        let config = attribute_config_with(&["hp"]);
        assert!(participant_attributes(None, &config).is_empty());
    }

    #[test]
    fn participant_attributes_reflects_active_buffs() {
        let config = attribute_config_with(&["hp"]);
        let mut character = character_with_attributes(&[("hp", 50, 100)]);
        character.active_effects.push(Effect {
            name: "buff".to_string(),
            effect_type: EffectType::AttributeBuff {
                attribute_id: "hp".to_string(),
                value: 20,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
            scope: EffectScope::default(),
        });

        let attributes = participant_attributes(Some(&character), &config);

        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].current, 70);
        assert_eq!(attributes[0].max, 100);
    }

    #[test]
    fn hp_attribute_id_reads_default_config() {
        assert_eq!(hp_attribute_id(&AttributeConfig::default_config()), "hp");
    }

    #[test]
    fn mp_attribute_id_reads_default_config() {
        assert_eq!(mp_attribute_id(&AttributeConfig::default_config()), "mp");
    }

    #[test]
    fn hp_attribute_id_falls_back_when_missing() {
        let config = AttributeConfig { attributes: vec![] };
        assert_eq!(hp_attribute_id(&config), "hp");
    }

    #[test]
    fn mp_attribute_id_falls_back_when_missing() {
        let config = AttributeConfig { attributes: vec![] };
        assert_eq!(mp_attribute_id(&config), "mp");
    }
}
