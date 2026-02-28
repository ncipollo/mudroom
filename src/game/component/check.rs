use serde::{Deserialize, Serialize};

use crate::game::next_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for Check {
    fn default() -> Self {
        Self {
            id: next_id(),
            attribute_id: 0,
            expected_value: 0,
        }
    }
}
