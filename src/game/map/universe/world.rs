use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::Dungeon;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub id: String,
    pub dungeons: HashMap<String, Dungeon>,
}

impl World {
    pub fn new(id: String) -> Self {
        Self {
            id,
            dungeons: HashMap::new(),
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new(uuid::Uuid::new_v4().to_string())
    }
}
