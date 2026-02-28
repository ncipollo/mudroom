use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::Room;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dungeon {
    pub id: String,
    pub rooms: HashMap<String, Room>,
}

impl Dungeon {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            rooms: HashMap::new(),
        }
    }
}

impl Default for Dungeon {
    fn default() -> Self {
        Self::new()
    }
}
