use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: i64,
    pub item_definition_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_serde_round_trip() {
        let item = Item {
            id: 1,
            item_definition_id: "health_tonic".to_string(),
        };
        let json = serde_json::to_string(&item).unwrap();
        let restored: Item = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, item);
    }
}
