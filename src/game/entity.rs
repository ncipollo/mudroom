use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::agent::entity_ai::EntityAI;
use crate::game::component::Ability;
use crate::game::component::Attribute;
use crate::game::component::FactionRelations;
use crate::game::component::Interaction;
use crate::game::component::Location;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Player,
    Character,
    Monster,
    Object,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: i64,
    pub entity_type: EntityType,
    pub location: Location,
    pub attributes: HashMap<String, Attribute>,
    pub interactions: Vec<Interaction>,
    #[serde(default)]
    pub innate_abilities: Vec<Ability>,
    pub config_id: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(skip)]
    pub ai: Option<EntityAI>,
    #[serde(default)]
    pub factions: HashSet<String>,
    #[serde(default)]
    pub faction_relations: FactionRelations,
}

impl Entity {
    pub fn new(id: i64, entity_type: EntityType, location: Location) -> Self {
        let mut factions = HashSet::new();
        if matches!(entity_type, EntityType::Player) {
            factions.insert("player".to_string());
        }
        if matches!(entity_type, EntityType::Monster) {
            factions.insert("monster".to_string());
        }
        Self {
            id,
            entity_type,
            location,
            attributes: HashMap::new(),
            interactions: Vec::new(),
            innate_abilities: Vec::new(),
            config_id: None,
            description: None,
            ai: None,
            factions,
            faction_relations: FactionRelations::default(),
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

    #[test]
    fn player_entity_has_player_faction() {
        let entity = Entity::new(1, EntityType::Player, test_location());
        assert!(entity.factions.contains("player"));
        assert_eq!(entity.factions.len(), 1);
    }

    #[test]
    fn monster_entity_has_monster_faction() {
        let entity = Entity::new(2, EntityType::Monster, test_location());
        assert!(entity.factions.contains("monster"));
        assert_eq!(entity.factions.len(), 1);
    }

    #[test]
    fn non_player_entity_has_empty_factions() {
        let entity = Entity::new(2, EntityType::Character, test_location());
        assert!(entity.factions.is_empty());
    }
}
