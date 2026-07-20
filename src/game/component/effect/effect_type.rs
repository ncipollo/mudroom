use crate::game::component::location::Location;
use crate::game::narration::VariableMap;
use serde::{Deserialize, Serialize};

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

    pub fn variable_map(&self) -> VariableMap {
        match self {
            EffectType::AttributeUpdate { value, .. } => VariableMap::new()
                .insert("value", value.to_string())
                .insert("abs_value", value.abs().to_string()),
            EffectType::AttributeShield { absorb_amount, .. } => VariableMap::new()
                .insert("value", absorb_amount.to_string())
                .insert("abs_value", absorb_amount.abs().to_string()),
            EffectType::EntitySpawn { .. } => VariableMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_map_attribute_update_provides_value_and_abs_value() {
        let vars = EffectType::AttributeUpdate {
            attribute_id: "hp".to_string(),
            value: -15,
        }
        .variable_map();
        assert_eq!(vars.get("value"), Some("-15"));
        assert_eq!(vars.get("abs_value"), Some("15"));
    }

    #[test]
    fn variable_map_attribute_shield_provides_value_and_abs_value() {
        let vars = EffectType::AttributeShield {
            attribute_id: "hp".to_string(),
            absorb_amount: 10,
        }
        .variable_map();
        assert_eq!(vars.get("value"), Some("10"));
        assert_eq!(vars.get("abs_value"), Some("10"));
    }

    #[test]
    fn variable_map_entity_spawn_is_empty() {
        let vars = EffectType::EntitySpawn {
            entity_id: "goblin".to_string(),
            location: None,
        }
        .variable_map();
        assert_eq!(vars.get("value"), None);
    }
}
