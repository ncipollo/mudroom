use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
