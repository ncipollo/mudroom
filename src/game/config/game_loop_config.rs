use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLoopConfig {
    pub tick_rate: u64,
    pub max_turn_ticks: u64,
    pub world_update_ticks: u64,
}

impl GameLoopConfig {
    pub fn default_config() -> Self {
        Self {
            tick_rate: 1000,
            max_turn_ticks: 30,
            world_update_ticks: 600,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let config = GameLoopConfig::default_config();
        assert_eq!(config.tick_rate, 1000);
        assert_eq!(config.max_turn_ticks, 30);
        assert_eq!(config.world_update_ticks, 600);
    }

    #[test]
    fn serde_round_trip() {
        let config = GameLoopConfig::default_config();
        let toml = toml::to_string(&config).unwrap();
        let restored: GameLoopConfig = toml::from_str(&toml).unwrap();
        assert_eq!(restored.tick_rate, config.tick_rate);
        assert_eq!(restored.max_turn_ticks, config.max_turn_ticks);
        assert_eq!(restored.world_update_ticks, config.world_update_ticks);
    }
}
