pub mod dungeon;
pub mod navigation;
pub mod room;
pub mod world;

pub use dungeon::Dungeon;
pub use navigation::Navigation;
pub use room::Room;
pub use world::World;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Universe {
    pub id: String,
    pub worlds: HashMap<String, World>,
}

impl Universe {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            worlds: HashMap::new(),
        }
    }
}

impl Default for Universe {
    fn default() -> Self {
        Self::new()
    }
}
