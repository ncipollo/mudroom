use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BattleAiType {
    #[default]
    None,
    SimpleRandom,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BattleAiConfig {
    #[serde(default)]
    pub ai_type: BattleAiType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battle_ai_type_default_is_none() {
        assert_eq!(BattleAiType::default(), BattleAiType::None);
    }

    #[test]
    fn battle_ai_config_default_is_none() {
        let config = BattleAiConfig::default();
        assert_eq!(config.ai_type, BattleAiType::None);
    }

    #[test]
    fn battle_ai_type_none_serde_round_trip() {
        let ai_type = BattleAiType::None;
        let json = serde_json::to_string(&ai_type).unwrap();
        assert_eq!(json, r#""none""#);
        let restored: BattleAiType = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, BattleAiType::None);
    }

    #[test]
    fn battle_ai_type_simple_random_serde_round_trip() {
        let ai_type = BattleAiType::SimpleRandom;
        let json = serde_json::to_string(&ai_type).unwrap();
        assert_eq!(json, r#""simple_random""#);
        let restored: BattleAiType = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, BattleAiType::SimpleRandom);
    }

    #[test]
    fn battle_ai_config_serde_round_trip() {
        let config = BattleAiConfig {
            ai_type: BattleAiType::SimpleRandom,
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: BattleAiConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.ai_type, BattleAiType::SimpleRandom);
    }

    #[test]
    fn battle_ai_config_deserializes_without_ai_type_field() {
        let json = r#"{}"#;
        let config: BattleAiConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.ai_type, BattleAiType::None);
    }

    #[test]
    fn battle_ai_config_toml_round_trip() {
        let toml = r#"ai_type = "simple_random""#;
        let config: BattleAiConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.ai_type, BattleAiType::SimpleRandom);
    }
}
