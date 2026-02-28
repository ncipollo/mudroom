use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Navigation {
    pub world_id: Option<String>,
    pub dungeon_id: Option<String>,
    pub room_id: Option<String>,
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
