use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Player,
    Character,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: i64,
    pub entity_type: EntityType,
}

impl Entity {
    pub fn new(id: i64, entity_type: EntityType) -> Self {
        Self { id, entity_type }
    }
}
