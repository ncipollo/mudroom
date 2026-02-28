use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub definition_id: String,
    pub min_value: i64,
    pub max_value: i64,
    pub current_value: i64,
}

impl Attribute {
    pub fn new(definition_id: String, min_value: i64, max_value: i64, current_value: i64) -> Self {
        Self {
            definition_id,
            min_value,
            max_value,
            current_value,
        }
    }
}
