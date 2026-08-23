use serde::{Deserialize, Serialize};

use crate::game::component::description::Description;
use crate::game::component::effect::Effect;
use crate::game::component::modifier::Modifier;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemUseType {
    Used,
    Passive,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum UseEffect {
    StatBoost(Modifier),
    ApplyEffect(Effect),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributeBonus {
    pub attribute_id: String,
    pub amount: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EquippedBonuses {
    #[serde(default)]
    pub attributes: Vec<AttributeBonus>,
    #[serde(default)]
    pub equipped: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemDefinition {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub description: Description,
    pub use_type: ItemUseType,
    pub item_type: String,
    #[serde(default)]
    pub equipped_bonuses: EquippedBonuses,
    #[serde(default)]
    pub use_effects: Vec<UseEffect>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::effect::{EffectDescription, EffectScope, EffectType, TriggerInfo};
    use crate::game::component::modifier::Operator;

    #[test]
    fn item_use_type_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&ItemUseType::Used).unwrap(),
            r#""used""#
        );
        assert_eq!(
            serde_json::to_string(&ItemUseType::Passive).unwrap(),
            r#""passive""#
        );
    }

    #[test]
    fn use_effect_stat_boost_serde_round_trip() {
        let use_effect = UseEffect::StatBoost(Modifier {
            attribute_id: "strength".to_string(),
            operator: Operator::Add,
            amount: 3,
        });
        let json = serde_json::to_string(&use_effect).unwrap();
        let restored: UseEffect = serde_json::from_str(&json).unwrap();
        assert_eq!(use_effect, restored);
    }

    #[test]
    fn use_effect_apply_effect_serde_round_trip() {
        let use_effect = UseEffect::ApplyEffect(Effect {
            name: "heal_hp".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: 20,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
            scope: EffectScope::default(),
        });
        let json = serde_json::to_string(&use_effect).unwrap();
        let restored: UseEffect = serde_json::from_str(&json).unwrap();
        assert_eq!(use_effect, restored);
    }

    #[test]
    fn attribute_bonus_serde_round_trip() {
        let bonus = AttributeBonus {
            attribute_id: "max_health".to_string(),
            amount: 10,
        };
        let json = serde_json::to_string(&bonus).unwrap();
        let restored: AttributeBonus = serde_json::from_str(&json).unwrap();
        assert_eq!(bonus, restored);
    }

    #[test]
    fn item_definition_serde_round_trip() {
        let def = ItemDefinition {
            id: "leather_vest".to_string(),
            name: "Leather Vest".to_string(),
            description: Description::new(Some("A simple protective vest.".to_string())),
            use_type: ItemUseType::Passive,
            item_type: "armor".to_string(),
            equipped_bonuses: EquippedBonuses {
                attributes: vec![AttributeBonus {
                    attribute_id: "defense".to_string(),
                    amount: 5,
                }],
                equipped: vec![],
            },
            use_effects: vec![],
        };
        let json = serde_json::to_string(&def).unwrap();
        let restored: ItemDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, def);
    }

    #[test]
    fn item_definition_missing_optional_fields_deserializes() {
        let json = r#"{
            "name": "Health Tonic",
            "description": null,
            "use_type": "used",
            "item_type": "medicine"
        }"#;
        let def: ItemDefinition = serde_json::from_str(json).unwrap();
        assert!(def.id.is_empty());
        assert!(def.equipped_bonuses.attributes.is_empty());
        assert!(def.equipped_bonuses.equipped.is_empty());
        assert!(def.use_effects.is_empty());
    }
}
