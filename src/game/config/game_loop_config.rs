use serde::{Deserialize, Serialize};

use crate::game::config::env_resolver::deserialize_env_u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLoopConfig {
    #[serde(deserialize_with = "deserialize_env_u64")]
    pub tick_rate_ms: u64,
    #[serde(deserialize_with = "deserialize_env_u64")]
    pub max_engage_ms: u64,
    #[serde(deserialize_with = "deserialize_env_u64")]
    pub world_update_ms: u64,
}

impl GameLoopConfig {
    pub fn default_config() -> Self {
        Self {
            tick_rate_ms: 1000,
            max_engage_ms: 300_000,
            world_update_ms: 600_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn default_config_has_expected_values() {
        let config = GameLoopConfig::default_config();
        assert_eq!(config.tick_rate_ms, 1000);
        assert_eq!(config.max_engage_ms, 300_000);
        assert_eq!(config.world_update_ms, 600_000);
    }

    #[test]
    fn serde_round_trip() {
        let config = GameLoopConfig::default_config();
        let toml = toml::to_string(&config).unwrap();
        let restored: GameLoopConfig = toml::from_str(&toml).unwrap();
        assert_eq!(restored.tick_rate_ms, config.tick_rate_ms);
        assert_eq!(restored.max_engage_ms, config.max_engage_ms);
        assert_eq!(restored.world_update_ms, config.world_update_ms);
    }

    #[test]
    fn game_loop_resolves_env_vars() {
        // SAFETY: env::set_var is unsafe because it is not thread-safe; acceptable in these single-threaded test contexts.
        unsafe {
            env::set_var("MUDROOM_TEST_TICK_RATE_MS", "500");
            env::set_var("MUDROOM_TEST_MAX_ENGAGE_MS", "15000");
            env::set_var("MUDROOM_TEST_WORLD_UPDATE_MS", "300000");
        }
        let toml = r#"
tick_rate_ms = "$MUDROOM_TEST_TICK_RATE_MS"
max_engage_ms = "$MUDROOM_TEST_MAX_ENGAGE_MS"
world_update_ms = "$MUDROOM_TEST_WORLD_UPDATE_MS"
"#;
        let config: GameLoopConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.tick_rate_ms, 500);
        assert_eq!(config.max_engage_ms, 15000);
        assert_eq!(config.world_update_ms, 300000);
        unsafe {
            env::remove_var("MUDROOM_TEST_TICK_RATE_MS");
            env::remove_var("MUDROOM_TEST_MAX_ENGAGE_MS");
            env::remove_var("MUDROOM_TEST_WORLD_UPDATE_MS");
        }
    }
}
