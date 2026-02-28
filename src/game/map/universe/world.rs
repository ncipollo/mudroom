use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::Dungeon;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub id: i64,
    pub dungeons: HashMap<i64, Dungeon>,
}

impl World {
    pub fn new(id: i64) -> Self {
        Self {
            id,
            dungeons: HashMap::new(),
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new(crate::game::next_id())
    }
}
