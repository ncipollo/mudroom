use serde::{Deserialize, Serialize};

use crate::game::component::Location;

/// The runtime state of one [`crate::game::RoomFeature`] placed in a specific room.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoomFeatureState {
    pub id: i64,
    pub feature_definition_id: String,
    pub location: Location,
    pub current_state: String,
    pub items: Vec<String>,
}

impl RoomFeatureState {
    pub fn new(
        id: i64,
        feature_definition_id: String,
        location: Location,
        current_state: String,
        items: Vec<String>,
    ) -> Self {
        Self {
            id,
            feature_definition_id,
            location,
            current_state,
            items,
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
    fn room_feature_state_new_stores_fields() {
        let state = RoomFeatureState::new(
            1,
            "chest".to_string(),
            test_location(),
            "closed".to_string(),
            vec!["medicine".to_string()],
        );
        assert_eq!(state.id, 1);
        assert_eq!(state.feature_definition_id, "chest");
        assert_eq!(state.location.world_id, "w1");
        assert_eq!(state.current_state, "closed");
        assert_eq!(state.items, vec!["medicine".to_string()]);
    }
}
