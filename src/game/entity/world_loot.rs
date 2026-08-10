use serde::{Deserialize, Serialize};

use crate::game::component::Location;
use crate::game::component::description::Description;
use crate::game::entity::Entity;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldLoot {
    pub id: i64,
    pub item_definition_id: String,
    pub location: Location,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Description,
}

impl Entity for WorldLoot {
    fn id(&self) -> i64 {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &Description {
        &self.description
    }

    fn location(&self) -> &Location {
        &self.location
    }
}

impl WorldLoot {
    pub fn new(
        id: i64,
        item_definition_id: String,
        location: Location,
        name: String,
        description: Description,
    ) -> Self {
        Self {
            id,
            item_definition_id,
            location,
            name,
            description,
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
    fn world_loot_new_stores_fields() {
        let loot = WorldLoot::new(
            1,
            "health_tonic".to_string(),
            test_location(),
            "Health Tonic".to_string(),
            Description::new(Some("A restorative brew.".to_string())),
        );
        assert_eq!(loot.id, 1);
        assert_eq!(loot.item_definition_id, "health_tonic");
        assert_eq!(loot.location.world_id, "w1");
        assert_eq!(loot.name, "Health Tonic");
    }

    #[test]
    fn world_loot_implements_entity_trait() {
        let loot = WorldLoot::new(
            1,
            "health_tonic".to_string(),
            test_location(),
            "Health Tonic".to_string(),
            Description::new(Some("A restorative brew.".to_string())),
        );
        assert_eq!(loot.id(), loot.id);
        assert_eq!(loot.name(), "Health Tonic");
        assert_eq!(
            loot.description().text.as_deref(),
            Some("A restorative brew.")
        );
        assert_eq!(loot.location().world_id, "w1");
    }
}
