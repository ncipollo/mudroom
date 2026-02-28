use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::game::component::AttributeDefinition;
use crate::game::component::attribute_definition::AttributeType;

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
        Self {
            attributes: vec![
                AttributeDefinition {
                    id: "hp".to_string(),
                    title: "Hit Points".to_string(),
                    description: "The amount of damage you can sustain before falling.".to_string(),
                    min_value: 0,
                    max_value: 999,
                    attribute_type: AttributeType::HP,
                },
                AttributeDefinition {
                    id: "mp".to_string(),
                    title: "Mana Points".to_string(),
                    description: "The magical energy available for spells and abilities."
                        .to_string(),
                    min_value: 0,
                    max_value: 999,
                    attribute_type: AttributeType::MP,
                },
                AttributeDefinition {
                    id: "level".to_string(),
                    title: "Level".to_string(),
                    description: "Your overall experience level.".to_string(),
                    min_value: 1,
                    max_value: 100,
                    attribute_type: AttributeType::Level,
                },
                AttributeDefinition {
                    id: "xp".to_string(),
                    title: "Experience Points".to_string(),
                    description: "Points accumulated through deeds and adventure.".to_string(),
                    min_value: 0,
                    max_value: i64::MAX,
                    attribute_type: AttributeType::XP,
                },
                AttributeDefinition {
                    id: "strength".to_string(),
                    title: "Strength".to_string(),
                    description: "Raw physical power and carrying capacity.".to_string(),
                    min_value: 1,
                    max_value: 20,
                    attribute_type: AttributeType::Stat,
                },
                AttributeDefinition {
                    id: "dexterity".to_string(),
                    title: "Dexterity".to_string(),
                    description: "Agility, reflexes, and hand-eye coordination.".to_string(),
                    min_value: 1,
                    max_value: 20,
                    attribute_type: AttributeType::Stat,
                },
                AttributeDefinition {
                    id: "constitution".to_string(),
                    title: "Constitution".to_string(),
                    description: "Endurance, stamina, and resistance to harm.".to_string(),
                    min_value: 1,
                    max_value: 20,
                    attribute_type: AttributeType::Stat,
                },
                AttributeDefinition {
                    id: "intelligence".to_string(),
                    title: "Intelligence".to_string(),
                    description: "Reasoning ability, memory, and arcane aptitude.".to_string(),
                    min_value: 1,
                    max_value: 20,
                    attribute_type: AttributeType::Stat,
                },
                AttributeDefinition {
                    id: "wisdom".to_string(),
                    title: "Wisdom".to_string(),
                    description: "Perception, intuition, and willpower.".to_string(),
                    min_value: 1,
                    max_value: 20,
                    attribute_type: AttributeType::Stat,
                },
                AttributeDefinition {
                    id: "charisma".to_string(),
                    title: "Charisma".to_string(),
                    description: "Force of personality, persuasiveness, and leadership."
                        .to_string(),
                    min_value: 1,
                    max_value: 20,
                    attribute_type: AttributeType::Stat,
                },
            ],
        }
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
        assert_eq!(config.attributes.len(), 10);
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
attribute_type = "hp"

[[attributes]]
id = "test_stat"
title = "Test Stat"
description = "A test stat."
min_value = 1
max_value = 20
attribute_type = "stat"
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(toml.as_bytes()).unwrap();
        let config = AttributeConfig::load(file.path()).unwrap();
        assert_eq!(config.attributes.len(), 2);
        assert_eq!(config.attributes[0].id, "test_hp");
        assert_eq!(config.attributes[1].id, "test_stat");
    }
}
