use crate::game::component::location::Location;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EffectScope {
    #[default]
    World,
    Battle,
    Conversation,
}

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
    AttributeShield {
        attribute_id: String,
        absorb_amount: i64,
    },
    EntitySpawn {
        entity_id: String,
        location: Option<Location>,
    },
}

impl EffectType {
    pub fn resolution_order(&self) -> u8 {
        match self {
            EffectType::AttributeShield { .. } => 0,
            EffectType::AttributeUpdate { .. } => 1,
            EffectType::EntitySpawn { .. } => 2,
        }
    }
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
    #[serde(default)]
    pub scope: EffectScope,
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
            scope: EffectScope::default(),
        };
        let json = serde_json::to_string(&effect).unwrap();
        let restored: Effect = serde_json::from_str(&json).unwrap();
        assert_eq!(effect, restored);
    }

    #[test]
    fn attribute_shield_serde_round_trip() {
        let effect = Effect {
            name: "damage_reduction".to_string(),
            effect_type: EffectType::AttributeShield {
                attribute_id: "hp".to_string(),
                absorb_amount: 5,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
            scope: EffectScope::default(),
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
            scope: EffectScope::default(),
        };
        let json = serde_json::to_string(&effect).unwrap();
        let restored: Effect = serde_json::from_str(&json).unwrap();
        assert_eq!(effect, restored);
    }

    #[test]
    fn effect_scope_serde_round_trip() {
        let effect = Effect {
            name: "poison".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -5,
            },
            trigger_info: TriggerInfo::OverTime {
                start: 0,
                end: Some(3),
                rate: 1,
            },
            description: EffectDescription::default(),
            scope: EffectScope::Battle,
        };
        let json = serde_json::to_string(&effect).unwrap();
        let restored: Effect = serde_json::from_str(&json).unwrap();
        assert_eq!(effect, restored);
        assert_eq!(restored.scope, EffectScope::Battle);
    }

    #[test]
    fn effect_scope_defaults_to_world_when_absent() {
        let json = r#"{
            "name": "regen",
            "effect_type": {"type": "attribute_update", "attribute_id": "hp", "value": 5},
            "trigger_info": {"type": "once"},
            "description": {}
        }"#;
        let effect: Effect = serde_json::from_str(json).unwrap();
        assert_eq!(effect.scope, EffectScope::World);
    }
}
