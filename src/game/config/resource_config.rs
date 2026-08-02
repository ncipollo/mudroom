use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::game::component::description::Description;
use crate::game::component::resource_definition::{ResourceDefinition, ResourceLifespan};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    pub resources: Vec<ResourceDefinition>,
}

impl ResourceConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn default_config() -> Self {
        Self {
            resources: vec![
                ResourceDefinition {
                    id: "action_points".to_string(),
                    name: "Action Points".to_string(),
                    description: Description::new(Some(
                        "Points spent to perform actions during an engagement.".to_string(),
                    )),
                    min_value: 0,
                    max_value: 100,
                    lifespan: ResourceLifespan::Engagement,
                },
                ResourceDefinition {
                    id: "gold".to_string(),
                    name: "Gold".to_string(),
                    description: Description::new(Some(
                        "Currency used for trading and purchasing items.".to_string(),
                    )),
                    min_value: 0,
                    max_value: i64::MAX,
                    lifespan: ResourceLifespan::Perpetual,
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
    fn default_config_has_expected_resources() {
        let config = ResourceConfig::default_config();
        let ids: Vec<&str> = config.resources.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"action_points"));
        assert!(ids.contains(&"gold"));
        assert_eq!(config.resources.len(), 2);
    }

    #[test]
    fn load_parses_toml() {
        let toml = r#"
[[resources]]
id = "test_mp"
name = "Test Mana"
description = "Test mana points."
min_value = 0
max_value = 100
lifespan = "perpetual"

[[resources]]
id = "test_stamina"
name = "Test Stamina"
description = "Test stamina."
min_value = 0
max_value = 50
lifespan = "engagement"
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(toml.as_bytes()).unwrap();
        let config = ResourceConfig::load(file.path()).unwrap();
        assert_eq!(config.resources.len(), 2);
        assert_eq!(config.resources[0].id, "test_mp");
        assert_eq!(config.resources[1].id, "test_stamina");
    }
}
