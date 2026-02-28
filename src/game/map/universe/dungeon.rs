use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::Room;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dungeon {
    pub id: String,
    pub rooms: HashMap<String, Room>,
}

impl Dungeon {
    pub fn new(id: String) -> Self {
        Self {
            id,
            rooms: HashMap::new(),
        }
    }
}

impl Default for Dungeon {
    fn default() -> Self {
        Self::new(uuid::Uuid::new_v4().to_string())
    }
}
