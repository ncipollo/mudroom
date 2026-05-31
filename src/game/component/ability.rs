use crate::game::component::effect::Effect;
use crate::game::engagement::EngagementType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Cost {
    Resource { attribute_id: String, amount: i64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ability {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub effects: Vec<Effect>,
    pub engagement_types: Vec<EngagementType>,
    pub costs: Vec<Cost>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::effect::{EffectDescription, EffectType, TriggerInfo};

    fn attack_effect() -> Effect {
        Effect {
            name: "physical_damage".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -10,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
        }
    }

    #[test]
    fn cost_resource_serde_round_trip() {
        let cost = Cost::Resource {
            attribute_id: "mp".to_string(),
            amount: 5,
        };
        let json = serde_json::to_string(&cost).unwrap();
        let restored: Cost = serde_json::from_str(&json).unwrap();
        assert_eq!(cost, restored);
    }

    #[test]
    fn ability_serde_round_trip() {
        let ability = Ability {
            id: "attack".to_string(),
            name: "Attack".to_string(),
            description: Some("A basic physical attack.".to_string()),
            effects: vec![attack_effect()],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![Cost::Resource {
                attribute_id: "stamina".to_string(),
                amount: 5,
            }],
        };
        let json = serde_json::to_string(&ability).unwrap();
        let restored: Ability = serde_json::from_str(&json).unwrap();
        assert_eq!(ability, restored);
    }

    #[test]
    fn ability_no_cost_serde_round_trip() {
        let ability = Ability {
            id: "heal".to_string(),
            name: "Heal".to_string(),
            description: None,
            effects: vec![Effect {
                name: "heal_hp".to_string(),
                effect_type: EffectType::AttributeUpdate {
                    attribute_id: "hp".to_string(),
                    value: 20,
                },
                trigger_info: TriggerInfo::Once,
                description: EffectDescription::default(),
            }],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![],
        };
        let json = serde_json::to_string(&ability).unwrap();
        let restored: Ability = serde_json::from_str(&json).unwrap();
        assert_eq!(ability, restored);
    }
}
