use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResetCondition {
    #[default]
    EachEngagementTurn,
    EndOfEngagement,
    Never,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttributeCategory {
    #[serde(rename = "life")]
    Life,
    #[serde(rename = "speed")]
    Speed,
    #[serde(rename = "general")]
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttributeType {
    #[serde(rename = "hp")]
    HP,
    #[serde(rename = "mp")]
    MP,
    #[serde(rename = "level")]
    Level,
    #[serde(rename = "xp")]
    XP,
    #[serde(rename = "stat")]
    Stat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDefinition {
    pub id: String,
    pub title: String,
    pub description: String,
    pub min_value: i64,
    pub max_value: i64,
    pub attribute_type: AttributeType,
    pub attribute_category: AttributeCategory,
    #[serde(default)]
    pub reset_condition: ResetCondition,
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
            attribute_type: AttributeType::HP,
            attribute_category: AttributeCategory::Life,
            reset_condition: ResetCondition::EndOfEngagement,
        };
        let json = serde_json::to_string(&def).unwrap();
        let restored: AttributeDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, def.id);
        assert_eq!(restored.title, def.title);
        assert_eq!(restored.min_value, def.min_value);
        assert_eq!(restored.max_value, def.max_value);
        assert_eq!(restored.reset_condition, def.reset_condition);
    }

    #[test]
    fn reset_condition_serializes_snake_case() {
        let cases = [
            (
                ResetCondition::EachEngagementTurn,
                r#""each_engagement_turn""#,
            ),
            (ResetCondition::EndOfEngagement, r#""end_of_engagement""#),
            (ResetCondition::Never, r#""never""#),
        ];
        for (condition, expected) in cases {
            assert_eq!(serde_json::to_string(&condition).unwrap(), expected);
        }
    }

    #[test]
    fn reset_condition_serde_round_trip() {
        for condition in [
            ResetCondition::EachEngagementTurn,
            ResetCondition::EndOfEngagement,
            ResetCondition::Never,
        ] {
            let json = serde_json::to_string(&condition).unwrap();
            let restored: ResetCondition = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, condition);
        }
    }

    #[test]
    fn attribute_definition_missing_reset_condition_defaults() {
        let json = r#"{
            "id": "strength",
            "title": "Strength",
            "description": "Raw power.",
            "min_value": 1,
            "max_value": 20,
            "attribute_type": "stat",
            "attribute_category": "general"
        }"#;
        let def: AttributeDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.reset_condition, ResetCondition::EachEngagementTurn);
    }

    #[test]
    fn attribute_category_serializes_lowercase() {
        let cases = [
            (AttributeCategory::Life, r#""life""#),
            (AttributeCategory::Speed, r#""speed""#),
            (AttributeCategory::General, r#""general""#),
        ];
        for (cat, expected) in cases {
            let s = serde_json::to_string(&cat).unwrap();
            assert_eq!(s, expected);
        }
    }

    #[test]
    fn attribute_category_serde_round_trip() {
        let def = AttributeDefinition {
            id: "strength".to_string(),
            title: "Strength".to_string(),
            description: "Raw power.".to_string(),
            min_value: 1,
            max_value: 20,
            attribute_type: AttributeType::Stat,
            attribute_category: AttributeCategory::General,
            reset_condition: ResetCondition::EachEngagementTurn,
        };
        let json = serde_json::to_string(&def).unwrap();
        let restored: AttributeDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, def.id);
        assert_eq!(restored.attribute_category, def.attribute_category);
    }

    #[test]
    fn attribute_type_serializes_lowercase() {
        let t = AttributeType::HP;
        let s = serde_json::to_string(&t).unwrap();
        assert_eq!(s, r#""hp""#);

        let t = AttributeType::MP;
        let s = serde_json::to_string(&t).unwrap();
        assert_eq!(s, r#""mp""#);

        let t = AttributeType::Level;
        let s = serde_json::to_string(&t).unwrap();
        assert_eq!(s, r#""level""#);

        let t = AttributeType::XP;
        let s = serde_json::to_string(&t).unwrap();
        assert_eq!(s, r#""xp""#);

        let t = AttributeType::Stat;
        let s = serde_json::to_string(&t).unwrap();
        assert_eq!(s, r#""stat""#);
    }
}
