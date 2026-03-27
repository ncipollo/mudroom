use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::game::config::game_loop_config::GameLoopConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnConfig {
    pub world_id: String,
    pub dungeon_id: String,
    pub room_id: String,
}

impl SpawnConfig {
    pub fn default_config() -> Self {
        Self {
            world_id: "default".to_string(),
            dungeon_id: "default".to_string(),
            room_id: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MudConfig {
    pub game_loop: GameLoopConfig,
    pub spawn: SpawnConfig,
}

impl MudConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn default_config() -> Self {
        Self {
            game_loop: GameLoopConfig::default_config(),
            spawn: SpawnConfig::default_config(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn default_config_has_expected_values() {
        let config = MudConfig::default_config();
        assert_eq!(config.game_loop.tick_rate, 1000);
        assert_eq!(config.game_loop.max_turn_ms, 30_000);
        assert_eq!(config.game_loop.world_update_ms, 600_000);
        assert_eq!(config.spawn.world_id, "default");
        assert_eq!(config.spawn.dungeon_id, "default");
        assert_eq!(config.spawn.room_id, "default");
    }

    #[test]
    fn load_parses_toml() {
        let toml = r#"
[game_loop]
tick_rate = 500
max_turn_ms = 15000
world_update_ms = 300000

[spawn]
world_id = "overworld"
dungeon_id = "town"
room_id = "square"
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(toml.as_bytes()).unwrap();
        let config = MudConfig::load(file.path()).unwrap();
        assert_eq!(config.game_loop.tick_rate, 500);
        assert_eq!(config.game_loop.max_turn_ms, 15000);
        assert_eq!(config.game_loop.world_update_ms, 300000);
        assert_eq!(config.spawn.world_id, "overworld");
        assert_eq!(config.spawn.dungeon_id, "town");
        assert_eq!(config.spawn.room_id, "square");
    }
}
