use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Navigation {
    pub world_id: Option<i64>,
    pub dungeon_id: Option<i64>,
    pub room_id: Option<i64>,
}

impl Navigation {
    pub fn new() -> Self {
        Self {
            world_id: None,
            dungeon_id: None,
            room_id: None,
        }
    }
}

impl Default for Navigation {
    fn default() -> Self {
        Self::new()
    }
}
