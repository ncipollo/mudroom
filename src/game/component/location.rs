use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub world_id: String,
    pub dungeon_id: String,
    pub room_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn location_serde_round_trip() {
        let loc = Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        };
        let json = serde_json::to_string(&loc).unwrap();
        let restored: Location = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.world_id, "w1");
        assert_eq!(restored.dungeon_id, "d1");
        assert_eq!(restored.room_id, "r1");
    }
}
