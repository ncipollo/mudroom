use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::Room;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dungeon {
    pub id: i64,
    pub rooms: HashMap<i64, Room>,
}

impl Dungeon {
    pub fn new(id: i64) -> Self {
        Self {
            id,
            rooms: HashMap::new(),
        }
    }
}

impl Default for Dungeon {
    fn default() -> Self {
        Self::new(crate::game::next_id())
    }
}
