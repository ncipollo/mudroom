use serde::{Deserialize, Serialize};

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
        };
        let json = serde_json::to_string(&def).unwrap();
        let restored: AttributeDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, def.id);
        assert_eq!(restored.title, def.title);
        assert_eq!(restored.min_value, def.min_value);
        assert_eq!(restored.max_value, def.max_value);
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
