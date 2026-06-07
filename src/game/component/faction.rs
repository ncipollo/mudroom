use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faction {
    pub id: String,
    pub name: String,
    pub description: String,
}

impl Faction {
    pub fn player() -> Self {
        Self {
            id: "player".to_string(),
            name: "Player".to_string(),
            description: "All player characters.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_faction_has_correct_fields() {
        let faction = Faction::player();
        assert_eq!(faction.id, "player");
        assert_eq!(faction.name, "Player");
    }

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
