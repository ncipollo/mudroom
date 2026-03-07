use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::game::config::game_loop_config::GameLoopConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MudConfig {
    pub game_loop: GameLoopConfig,
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
        assert_eq!(config.game_loop.max_turn_ticks, 30);
        assert_eq!(config.game_loop.world_update_ticks, 600);
    }

    #[test]
    fn load_parses_toml() {
        let toml = r#"
[game_loop]
tick_rate = 500
max_turn_ticks = 15
world_update_ticks = 300
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(toml.as_bytes()).unwrap();
        let config = MudConfig::load(file.path()).unwrap();
        assert_eq!(config.game_loop.tick_rate, 500);
        assert_eq!(config.game_loop.max_turn_ticks, 15);
        assert_eq!(config.game_loop.world_update_ticks, 300);
    }
}
