use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faction {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faction_serde_round_trip() {
        let faction = Faction {
            id: "player".to_string(),
            name: "Player".to_string(),
            description: "All player characters.".to_string(),
        };
        let json = serde_json::to_string(&faction).unwrap();
        let restored: Faction = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, faction.id);
        assert_eq!(restored.name, faction.name);
        assert_eq!(restored.description, faction.description);
    }
}
