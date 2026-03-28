use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Check {
    pub id: i64,
    pub attribute_id: i64,
    pub expected_value: i64,
}

impl Check {
    pub fn new(id: i64, attribute_id: i64, expected_value: i64) -> Self {
        Self {
            id,
            attribute_id,
            expected_value,
        }
    }
}
