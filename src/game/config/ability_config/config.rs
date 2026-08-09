use crate::game::component::description::Description;
use crate::game::component::effect::Effect;
use crate::game::engagement::EngagementType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AbilityTargetType {
    #[default]
    Opponent,
    Allies,
    #[serde(rename = "self")]
    SelfTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AbilityRole {
    #[default]
    Attack,
    Defend,
    World,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Cost {
    Resource { resource_id: String, amount: i64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Modifier {
    pub attribute_id: String,
    pub operator: Operator,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AbilityReference(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ability {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub description: Description,
    pub effects: Vec<Effect>,
    pub engagement_types: Vec<EngagementType>,
    pub costs: Vec<Cost>,
    #[serde(default)]
    pub modifiers: Vec<Modifier>,
    #[serde(default)]
    pub role: AbilityRole,
    #[serde(default)]
    pub targets: Vec<AbilityTargetType>,
    #[serde(default)]
    pub action_text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::effect::{EffectDescription, EffectScope, EffectType, TriggerInfo};

    fn attack_effect() -> Effect {
        Effect {
            name: "physical_damage".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -10,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
            scope: EffectScope::default(),
        }
    }

    #[test]
    fn cost_resource_serde_round_trip() {
        let cost = Cost::Resource {
            resource_id: "mp".to_string(),
            amount: 5,
        };
        let json = serde_json::to_string(&cost).unwrap();
        let restored: Cost = serde_json::from_str(&json).unwrap();
        assert_eq!(cost, restored);
    }

    #[test]
    fn operator_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&Operator::Add).unwrap(), r#""add""#);
        assert_eq!(
            serde_json::to_string(&Operator::Subtract).unwrap(),
            r#""subtract""#
        );
        assert_eq!(
            serde_json::to_string(&Operator::Multiply).unwrap(),
            r#""multiply""#
        );
        assert_eq!(
            serde_json::to_string(&Operator::Divide).unwrap(),
            r#""divide""#
        );
    }

    #[test]
    fn modifier_serde_round_trip() {
        let modifier = Modifier {
            attribute_id: "strength".to_string(),
            operator: Operator::Multiply,
        };
        let json = serde_json::to_string(&modifier).unwrap();
        let restored: Modifier = serde_json::from_str(&json).unwrap();
        assert_eq!(modifier, restored);
    }

    #[test]
    fn ability_serde_round_trip() {
        let ability = Ability {
            id: "attack".to_string(),
            name: "Attack".to_string(),
            description: Description::new(Some("A basic physical attack.".to_string())),
            effects: vec![attack_effect()],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![Cost::Resource {
                resource_id: "stamina".to_string(),
                amount: 5,
            }],
            modifiers: vec![],
            role: AbilityRole::Attack,
            targets: vec![],
            action_text: None,
        };
        let json = serde_json::to_string(&ability).unwrap();
        let restored: Ability = serde_json::from_str(&json).unwrap();
        assert_eq!(ability, restored);
    }

    #[test]
    fn ability_with_modifiers_serde_round_trip() {
        let ability = Ability {
            id: "attack".to_string(),
            name: "Attack".to_string(),
            description: Description::new(Some("A strength-scaled attack.".to_string())),
            effects: vec![attack_effect()],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![Cost::Resource {
                resource_id: "stamina".to_string(),
                amount: 5,
            }],
            modifiers: vec![
                Modifier {
                    attribute_id: "strength".to_string(),
                    operator: Operator::Multiply,
                },
                Modifier {
                    attribute_id: "dexterity".to_string(),
                    operator: Operator::Add,
                },
            ],
            role: AbilityRole::Attack,
            targets: vec![],
            action_text: None,
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
            description: Description::default(),
            effects: vec![Effect {
                name: "heal_hp".to_string(),
                effect_type: EffectType::AttributeUpdate {
                    attribute_id: "hp".to_string(),
                    value: 20,
                },
                trigger_info: TriggerInfo::Once,
                description: EffectDescription::default(),
                scope: EffectScope::default(),
            }],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![],
            modifiers: vec![],
            role: AbilityRole::Attack,
            targets: vec![],
            action_text: None,
        };
        let json = serde_json::to_string(&ability).unwrap();
        let restored: Ability = serde_json::from_str(&json).unwrap();
        assert_eq!(ability, restored);
    }

    #[test]
    fn ability_missing_modifiers_field_deserializes() {
        let json = r#"{
            "id": "heal",
            "name": "Heal",
            "description": null,
            "effects": [],
            "engagement_types": [],
            "costs": []
        }"#;
        let ability: Ability = serde_json::from_str(json).unwrap();
        assert!(ability.modifiers.is_empty());
    }

    #[test]
    fn ability_target_type_serde_round_trip() {
        assert_eq!(
            serde_json::to_string(&AbilityTargetType::Opponent).unwrap(),
            r#""opponent""#
        );
        assert_eq!(
            serde_json::to_string(&AbilityTargetType::Allies).unwrap(),
            r#""allies""#
        );
        assert_eq!(
            serde_json::to_string(&AbilityTargetType::SelfTarget).unwrap(),
            r#""self""#
        );
        let restored: AbilityTargetType = serde_json::from_str(r#""self""#).unwrap();
        assert_eq!(restored, AbilityTargetType::SelfTarget);
    }

    #[test]
    fn ability_with_targets_serde_round_trip() {
        let ability = Ability {
            id: "attack".to_string(),
            name: "Attack".to_string(),
            description: Description::default(),
            effects: vec![attack_effect()],
            engagement_types: vec![],
            costs: vec![],
            modifiers: vec![],
            role: AbilityRole::Attack,
            targets: vec![AbilityTargetType::Opponent],
            action_text: None,
        };
        let json = serde_json::to_string(&ability).unwrap();
        let restored: Ability = serde_json::from_str(&json).unwrap();
        assert_eq!(ability, restored);
    }

    #[test]
    fn ability_missing_targets_field_deserializes() {
        let json = r#"{
            "id": "attack",
            "name": "Attack",
            "description": null,
            "effects": [],
            "engagement_types": [],
            "costs": []
        }"#;
        let ability: Ability = serde_json::from_str(json).unwrap();
        assert!(ability.targets.is_empty());
    }

    #[test]
    fn ability_with_action_text_serde_round_trip() {
        let ability = Ability {
            id: "swing_ax".to_string(),
            name: "Swing Ax".to_string(),
            description: Description::default(),
            effects: vec![attack_effect()],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![],
            modifiers: vec![],
            role: AbilityRole::Attack,
            targets: vec![AbilityTargetType::Opponent],
            action_text: Some("{{character}} swings ax at {{target}} for {{effect}}".to_string()),
        };
        let json = serde_json::to_string(&ability).unwrap();
        let restored: Ability = serde_json::from_str(&json).unwrap();
        assert_eq!(ability, restored);
    }

    #[test]
    fn ability_missing_action_text_deserializes_as_none() {
        let json = r#"{
            "id": "attack",
            "name": "Attack",
            "description": null,
            "effects": [],
            "engagement_types": [],
            "costs": []
        }"#;
        let ability: Ability = serde_json::from_str(json).unwrap();
        assert!(ability.action_text.is_none());
    }
}
