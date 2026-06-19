use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::agent::entity_ai::EntityAI;
use crate::game::component::Ability;
use crate::game::component::Attribute;
use crate::game::component::FactionRelations;
use crate::game::component::Interaction;
use crate::game::component::Location;
use crate::game::component::effect::Effect;
use crate::game::config::BattleAiConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Player,
    Character,
    Enemy,
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
    pub battle_ai: BattleAiConfig,
    #[serde(default)]
    pub factions: HashSet<String>,
    #[serde(default)]
    pub faction_relations: FactionRelations,
    #[serde(skip)]
    pub active_effects: Vec<Effect>,
}

impl Entity {
    pub fn new(id: i64, entity_type: EntityType, location: Location) -> Self {
        let mut factions = HashSet::new();
        if matches!(entity_type, EntityType::Player) {
            factions.insert("player".to_string());
        }
        if matches!(entity_type, EntityType::Enemy) {
            factions.insert("enemy".to_string());
        }
        let faction_relations = match entity_type {
            EntityType::Player => FactionRelations::default_for_player(),
            EntityType::Enemy => FactionRelations::default_for_enemy(),
            _ => FactionRelations::default(),
        };
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
            battle_ai: BattleAiConfig::default(),
            factions,
            faction_relations,
            active_effects: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::faction_relations::FactionRelation;

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
    fn enemy_entity_has_enemy_faction() {
        let entity = Entity::new(2, EntityType::Enemy, test_location());
        assert!(entity.factions.contains("enemy"));
        assert_eq!(entity.factions.len(), 1);
    }

    #[test]
    fn non_player_entity_has_empty_factions() {
        let entity = Entity::new(2, EntityType::Character, test_location());
        assert!(entity.factions.is_empty());
    }

    #[test]
    fn player_entity_has_default_faction_relations() {
        let entity = Entity::new(1, EntityType::Player, test_location());
        assert_eq!(
            entity.faction_relations.player_relation(),
            &FactionRelation::Friendly
        );
        assert_eq!(
            entity.faction_relations.enemy_relation(),
            &FactionRelation::Hostile
        );
    }

    #[test]
    fn enemy_entity_has_default_faction_relations() {
        let entity = Entity::new(2, EntityType::Enemy, test_location());
        assert_eq!(
            entity.faction_relations.player_relation(),
            &FactionRelation::Hostile
        );
        assert_eq!(
            entity.faction_relations.enemy_relation(),
            &FactionRelation::NonInteractive
        );
    }

    #[test]
    fn character_entity_has_empty_faction_relations() {
        let entity = Entity::new(3, EntityType::Character, test_location());
        assert!(entity.faction_relations.factions.is_empty());
    }
}
