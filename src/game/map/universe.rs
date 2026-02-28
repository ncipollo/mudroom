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
    pub id: i64,
    pub worlds: HashMap<i64, World>,
}

impl Universe {
    pub fn new(id: i64) -> Self {
        Self {
            id,
            worlds: HashMap::new(),
        }
    }
}

impl Default for Universe {
    fn default() -> Self {
        Self::new(crate::game::next_id())
    }
}
