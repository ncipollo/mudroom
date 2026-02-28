use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::game::component::Attribute;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Player,
    Character,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: i64,
    pub entity_type: EntityType,
    pub attributes: HashMap<String, Attribute>,
}

impl Entity {
    pub fn new(id: i64, entity_type: EntityType) -> Self {
        Self {
            id,
            entity_type,
            attributes: HashMap::new(),
        }
    }
}
