use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::game::component::faction::Faction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionConfig {
    pub factions: Vec<Faction>,
}

impl FactionConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn default_config() -> Self {
        Self {
            factions: vec![
                Faction::player(),
                Faction {
                    id: "monster".to_string(),
                    name: "Monster".to_string(),
                    description: "Hostile creatures of the world.".to_string(),
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
    fn default_config_has_expected_factions() {
        let config = FactionConfig::default_config();
        let ids: Vec<&str> = config.factions.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"player"));
        assert!(ids.contains(&"monster"));
        assert_eq!(config.factions.len(), 2);
    }

    #[test]
    fn load_parses_toml() {
        let toml = r#"
[[factions]]
id = "guard"
name = "Guard"
description = "City guards."

[[factions]]
id = "bandit"
name = "Bandit"
description = "Outlaws and brigands."
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(toml.as_bytes()).unwrap();
        let config = FactionConfig::load(file.path()).unwrap();
        assert_eq!(config.factions.len(), 2);
        assert_eq!(config.factions[0].id, "guard");
        assert_eq!(config.factions[1].id, "bandit");
    }
}
