use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::game::component::attribute_definition::OnZeroTrigger;
use crate::game::component::{Attribute, AttributeDefinition};
use crate::game::config::character_config::StartingAttribute;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeConfig {
    pub attributes: Vec<AttributeDefinition>,
}

impl AttributeConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn default_config() -> Self {
        let mut attributes = default_life_attributes();
        attributes.extend(default_stat_attributes());
        attributes.push(default_speed_attribute());
        Self { attributes }
    }

    /// Whether `attribute_id` can be the target of an `AttributeUpdate` effect. Fails open (`true`)
    /// for an id with no matching definition, preserving prior behavior for undeclared attributes.
    pub fn is_updatable(&self, attribute_id: &str) -> bool {
        self.attributes
            .iter()
            .find(|a| a.id == attribute_id)
            .is_none_or(|a| a.updatable)
    }

    /// Builds a character's starting attribute set: every declared `AttributeDefinition` defaults
    /// to its own `min_value`, then each `overrides` entry replaces just its own id. An override
    /// id with no matching definition is still inserted, failing open like `is_updatable`.
    pub fn starting_attributes(
        &self,
        overrides: &[StartingAttribute],
    ) -> HashMap<String, Attribute> {
        let mut attrs: HashMap<String, Attribute> = self
            .attributes
            .iter()
            .map(|def| {
                (
                    def.id.clone(),
                    Attribute::new(def.id.clone(), def.min_value, def.max_value, def.min_value),
                )
            })
            .collect();
        for sa in overrides {
            attrs.insert(
                sa.definition_id.clone(),
                Attribute::new(
                    sa.definition_id.clone(),
                    sa.min_value,
                    sa.max_value,
                    sa.current_value,
                ),
            );
        }
        attrs
    }
}

fn default_life_attributes() -> Vec<AttributeDefinition> {
    vec![
        AttributeDefinition {
            id: "hp".to_string(),
            title: "Hit Points".to_string(),
            description: "The amount of damage you can sustain before falling.".to_string(),
            min_value: 0,
            max_value: 999,
            on_zero: OnZeroTrigger::Death,
            updatable: true,
        },
        AttributeDefinition {
            id: "mp".to_string(),
            title: "Mana Points".to_string(),
            description: "The magical energy available for spells and abilities.".to_string(),
            min_value: 0,
            max_value: 999,
            on_zero: OnZeroTrigger::None,
            updatable: true,
        },
        AttributeDefinition {
            id: "level".to_string(),
            title: "Level".to_string(),
            description: "Your overall experience level.".to_string(),
            min_value: 1,
            max_value: 100,
            on_zero: OnZeroTrigger::None,
            updatable: true,
        },
        AttributeDefinition {
            id: "xp".to_string(),
            title: "Experience Points".to_string(),
            description: "Points accumulated through deeds and adventure.".to_string(),
            min_value: 0,
            max_value: i64::MAX,
            on_zero: OnZeroTrigger::None,
            updatable: true,
        },
    ]
}

fn default_stat_attributes() -> Vec<AttributeDefinition> {
    vec![
        AttributeDefinition {
            id: "strength".to_string(),
            title: "Strength".to_string(),
            description: "Raw physical power and carrying capacity.".to_string(),
            min_value: 1,
            max_value: 20,
            on_zero: OnZeroTrigger::None,
            updatable: true,
        },
        AttributeDefinition {
            id: "dexterity".to_string(),
            title: "Dexterity".to_string(),
            description: "Agility, reflexes, and hand-eye coordination.".to_string(),
            min_value: 1,
            max_value: 20,
            on_zero: OnZeroTrigger::None,
            updatable: true,
        },
        AttributeDefinition {
            id: "constitution".to_string(),
            title: "Constitution".to_string(),
            description: "Endurance, stamina, and resistance to harm.".to_string(),
            min_value: 1,
            max_value: 20,
            on_zero: OnZeroTrigger::None,
            updatable: true,
        },
        AttributeDefinition {
            id: "intelligence".to_string(),
            title: "Intelligence".to_string(),
            description: "Reasoning ability, memory, and arcane aptitude.".to_string(),
            min_value: 1,
            max_value: 20,
            on_zero: OnZeroTrigger::None,
            updatable: true,
        },
        AttributeDefinition {
            id: "wisdom".to_string(),
            title: "Wisdom".to_string(),
            description: "Perception, intuition, and willpower.".to_string(),
            min_value: 1,
            max_value: 20,
            on_zero: OnZeroTrigger::None,
            updatable: true,
        },
        AttributeDefinition {
            id: "charisma".to_string(),
            title: "Charisma".to_string(),
            description: "Force of personality, persuasiveness, and leadership.".to_string(),
            min_value: 1,
            max_value: 20,
            on_zero: OnZeroTrigger::None,
            updatable: true,
        },
    ]
}

/// The attribute `BattleConfig::default_config`'s `turn_order_attributes` names by default, so a
/// fresh mud gets working turn order without any explicit configuration.
fn default_speed_attribute() -> AttributeDefinition {
    AttributeDefinition {
        id: "speed".to_string(),
        title: "Speed".to_string(),
        description: "Determines turn order in battle; faster entities act first.".to_string(),
        min_value: 1,
        max_value: 20,
        on_zero: OnZeroTrigger::None,
        updatable: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn default_config_has_expected_attributes() {
        let config = AttributeConfig::default_config();
        let ids: Vec<&str> = config.attributes.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"hp"));
        assert!(ids.contains(&"mp"));
        assert!(ids.contains(&"level"));
        assert!(ids.contains(&"xp"));
        assert!(ids.contains(&"strength"));
        assert!(ids.contains(&"dexterity"));
        assert!(ids.contains(&"constitution"));
        assert!(ids.contains(&"intelligence"));
        assert!(ids.contains(&"wisdom"));
        assert!(ids.contains(&"charisma"));
        assert!(ids.contains(&"speed"));
        assert_eq!(config.attributes.len(), 11);
    }

    #[test]
    fn default_config_attributes_are_all_updatable() {
        let config = AttributeConfig::default_config();
        assert!(config.attributes.iter().all(|a| a.updatable));
    }

    #[test]
    fn is_updatable_reads_definition_flag() {
        let mut config = AttributeConfig::default_config();
        config
            .attributes
            .iter_mut()
            .find(|a| a.id == "hp")
            .unwrap()
            .updatable = false;
        assert!(!config.is_updatable("hp"));
        assert!(config.is_updatable("mp"));
    }

    #[test]
    fn is_updatable_fails_open_for_unknown_attribute() {
        let config = AttributeConfig::default_config();
        assert!(config.is_updatable("nonexistent"));
    }

    #[test]
    fn starting_attributes_defaults_every_definition_to_min_value() {
        let config = AttributeConfig::default_config();
        let attrs = config.starting_attributes(&[]);
        assert_eq!(attrs.len(), config.attributes.len());
        for def in &config.attributes {
            let attr = &attrs[&def.id];
            assert_eq!(attr.min_value, def.min_value);
            assert_eq!(attr.max_value, def.max_value);
            assert_eq!(attr.current_value, def.min_value);
        }
    }

    #[test]
    fn starting_attributes_override_replaces_only_its_own_id() {
        let config = AttributeConfig::default_config();
        let overrides = [StartingAttribute {
            definition_id: "hp".to_string(),
            min_value: 0,
            max_value: 120,
            current_value: 120,
        }];
        let attrs = config.starting_attributes(&overrides);
        assert_eq!(attrs["hp"], Attribute::new("hp".to_string(), 0, 120, 120));
        let mp_def = config.attributes.iter().find(|a| a.id == "mp").unwrap();
        assert_eq!(attrs["mp"].current_value, mp_def.min_value);
    }

    #[test]
    fn starting_attributes_inserts_unknown_override_id() {
        let config = AttributeConfig::default_config();
        let overrides = [StartingAttribute {
            definition_id: "nonexistent".to_string(),
            min_value: 0,
            max_value: 10,
            current_value: 5,
        }];
        let attrs = config.starting_attributes(&overrides);
        assert_eq!(
            attrs["nonexistent"],
            Attribute::new("nonexistent".to_string(), 0, 10, 5)
        );
    }

    #[test]
    fn default_config_only_hp_triggers_death() {
        let config = AttributeConfig::default_config();
        let on_zero = |id: &str| {
            config
                .attributes
                .iter()
                .find(|a| a.id == id)
                .unwrap()
                .on_zero
        };
        assert_eq!(on_zero("hp"), OnZeroTrigger::Death);
        for id in [
            "mp",
            "level",
            "xp",
            "strength",
            "dexterity",
            "constitution",
            "intelligence",
            "wisdom",
            "charisma",
            "speed",
        ] {
            assert_eq!(on_zero(id), OnZeroTrigger::None);
        }
    }

    #[test]
    fn load_parses_toml() {
        let toml = r#"
[[attributes]]
id = "test_hp"
title = "Test HP"
description = "Test hit points."
min_value = 0
max_value = 100
on_zero = "death"

[[attributes]]
id = "test_stat"
title = "Test Stat"
description = "A test stat."
min_value = 1
max_value = 20
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(toml.as_bytes()).unwrap();
        let config = AttributeConfig::load(file.path()).unwrap();
        assert_eq!(config.attributes.len(), 2);
        assert_eq!(config.attributes[0].id, "test_hp");
        assert_eq!(config.attributes[0].on_zero, OnZeroTrigger::Death);
        assert_eq!(config.attributes[1].id, "test_stat");
        assert_eq!(config.attributes[1].on_zero, OnZeroTrigger::None);
    }
}
