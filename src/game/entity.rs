use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::game::component::Attribute;
use crate::game::component::Location;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Player,
    Character,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: i64,
    pub entity_type: EntityType,
    pub location: Location,
    pub attributes: HashMap<String, Attribute>,
}

impl Entity {
    pub fn new(id: i64, entity_type: EntityType, location: Location) -> Self {
        Self {
            id,
            entity_type,
            location,
            attributes: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    #[test]
    fn entity_new_stores_location() {
        let loc = test_location();
        let entity = Entity::new(1, EntityType::Player, loc);
        assert_eq!(entity.location.world_id, "w1");
        assert_eq!(entity.location.dungeon_id, "d1");
        assert_eq!(entity.location.room_id, "r1");
    }
}
