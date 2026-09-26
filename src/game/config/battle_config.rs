use serde::{Deserialize, Serialize};

fn default_turn_order_attributes() -> Vec<String> {
    vec!["speed".to_string()]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleConfig {
    #[serde(default = "default_turn_order_attributes")]
    pub turn_order_attributes: Vec<String>,
}

impl BattleConfig {
    pub fn default_config() -> Self {
        Self {
            turn_order_attributes: default_turn_order_attributes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_speed() {
        let config = BattleConfig::default_config();
        assert_eq!(config.turn_order_attributes, vec!["speed"]);
    }

    #[test]
    fn missing_turn_order_attributes_defaults_to_speed() {
        let config: BattleConfig = toml::from_str("").unwrap();
        assert_eq!(config.turn_order_attributes, vec!["speed"]);
    }

    #[test]
    fn parses_turn_order_attributes() {
        let config: BattleConfig =
            toml::from_str("turn_order_attributes = [\"agility\", \"dex\"]").unwrap();
        assert_eq!(config.turn_order_attributes, vec!["agility", "dex"]);
    }
}
