use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OnZeroTrigger {
    #[default]
    None,
    Death,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDefinition {
    pub id: String,
    pub title: String,
    pub description: String,
    pub min_value: i64,
    pub max_value: i64,
    #[serde(default)]
    pub on_zero: OnZeroTrigger,
    /// Whether this attribute can be the target of an `AttributeUpdate` effect. Defaults to
    /// `true` so existing configs (hp, mp, xp, level, stats) keep behaving as before.
    #[serde(default = "default_updatable")]
    pub updatable: bool,
}

fn default_updatable() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_definition_serde_round_trip() {
        let def = AttributeDefinition {
            id: "hp".to_string(),
            title: "Hit Points".to_string(),
            description: "The amount of damage you can take.".to_string(),
            min_value: 0,
            max_value: 100,
            on_zero: OnZeroTrigger::Death,
            updatable: true,
        };
        let json = serde_json::to_string(&def).unwrap();
        let restored: AttributeDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, def.id);
        assert_eq!(restored.title, def.title);
        assert_eq!(restored.min_value, def.min_value);
        assert_eq!(restored.max_value, def.max_value);
        assert_eq!(restored.on_zero, def.on_zero);
        assert_eq!(restored.updatable, def.updatable);
    }

    #[test]
    fn attribute_definition_missing_optional_fields_defaults() {
        let json = r#"{
            "id": "strength",
            "title": "Strength",
            "description": "Raw power.",
            "min_value": 1,
            "max_value": 20
        }"#;
        let def: AttributeDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.on_zero, OnZeroTrigger::None);
        assert!(def.updatable);
    }

    #[test]
    fn attribute_definition_updatable_false_round_trips() {
        let def = AttributeDefinition {
            id: "derived_stat".to_string(),
            title: "Derived Stat".to_string(),
            description: "Computed from other attributes.".to_string(),
            min_value: 0,
            max_value: 100,
            on_zero: OnZeroTrigger::None,
            updatable: false,
        };
        let json = serde_json::to_string(&def).unwrap();
        let restored: AttributeDefinition = serde_json::from_str(&json).unwrap();
        assert!(!restored.updatable);
    }

    #[test]
    fn on_zero_trigger_serializes_snake_case() {
        let cases = [
            (OnZeroTrigger::None, r#""none""#),
            (OnZeroTrigger::Death, r#""death""#),
        ];
        for (trigger, expected) in cases {
            assert_eq!(serde_json::to_string(&trigger).unwrap(), expected);
        }
    }

    #[test]
    fn on_zero_trigger_serde_round_trip() {
        for trigger in [OnZeroTrigger::None, OnZeroTrigger::Death] {
            let json = serde_json::to_string(&trigger).unwrap();
            let restored: OnZeroTrigger = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, trigger);
        }
    }
}
