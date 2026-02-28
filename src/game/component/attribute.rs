use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub id: i64,
    pub name: String,
    pub value: i64,
}

impl Attribute {
    pub fn new(id: i64, name: String, value: i64) -> Self {
        Self { id, name, value }
    }
}
