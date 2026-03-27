use crate::game::component::location::Location;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum TriggerInfo {
    OverTime {
        start: u64,
        end: Option<u64>,
        rate: u64,
    },
    Once,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum EffectType {
    AttributeUpdate {
        attribute_id: String,
        value: i64,
    },
    EntitySpawn {
        entity_id: String,
        location: Option<Location>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EffectDescription {
    pub start_description: Option<String>,
    pub end_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Effect {
    pub name: String,
    pub effect_type: EffectType,
    pub trigger_info: TriggerInfo,
    #[serde(default)]
    pub description: EffectDescription,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_serde_round_trip() {
        let effect = Effect {
            name: "heal_over_time".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: 5,
            },
            trigger_info: TriggerInfo::OverTime {
                start: 0,
                end: Some(10),
                rate: 1,
            },
            description: EffectDescription {
                start_description: Some("You feel better.".to_string()),
                end_description: None,
            },
        };
        let json = serde_json::to_string(&effect).unwrap();
        let restored: Effect = serde_json::from_str(&json).unwrap();
        assert_eq!(effect, restored);
    }

    #[test]
    fn once_trigger_serde_round_trip() {
        let effect = Effect {
            name: "spawn_entity".to_string(),
            effect_type: EffectType::EntitySpawn {
                entity_id: "goblin".to_string(),
                location: None,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
        };
        let json = serde_json::to_string(&effect).unwrap();
        let restored: Effect = serde_json::from_str(&json).unwrap();
        assert_eq!(effect, restored);
    }
}
