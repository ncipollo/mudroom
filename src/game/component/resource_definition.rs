use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceLifespan {
    Perpetual,
    Engagement,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub min_value: i64,
    pub max_value: i64,
    pub lifespan: ResourceLifespan,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_lifespan_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&ResourceLifespan::Perpetual).unwrap(),
            r#""perpetual""#
        );
        assert_eq!(
            serde_json::to_string(&ResourceLifespan::Engagement).unwrap(),
            r#""engagement""#
        );
    }

    #[test]
    fn resource_definition_serde_round_trip() {
        let def = ResourceDefinition {
            id: "mp".to_string(),
            name: "Mana Points".to_string(),
            description: "Magical energy for spells.".to_string(),
            min_value: 0,
            max_value: 999,
            lifespan: ResourceLifespan::Perpetual,
        };
        let json = serde_json::to_string(&def).unwrap();
        let restored: ResourceDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, def);
    }
}
