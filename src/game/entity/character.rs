use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::agent::entity_ai::EntityAI;
use crate::game::component::Ability;
use crate::game::component::Attribute;
use crate::game::component::FactionRelations;
use crate::game::component::Interaction;
use crate::game::component::Inventory;
use crate::game::component::Location;
use crate::game::component::description::Description;
use crate::game::component::effect::Effect;
use crate::game::config::BattleAiConfig;
use crate::game::entity::Entity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CharacterType {
    Player,
    Character,
    Enemy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: i64,
    pub character_type: CharacterType,
    pub location: Location,
    pub attributes: HashMap<String, Attribute>,
    pub interactions: Vec<Interaction>,
    #[serde(default)]
    pub innate_abilities: Vec<Ability>,
    pub config_id: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Description,
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
    #[serde(default)]
    pub inventory: Inventory,
}

impl Entity for Character {
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

impl Character {
    pub fn new(id: i64, character_type: CharacterType, location: Location) -> Self {
        let mut factions = HashSet::new();
        if matches!(character_type, CharacterType::Player) {
            factions.insert("player".to_string());
        }
        if matches!(character_type, CharacterType::Enemy) {
            factions.insert("enemy".to_string());
        }
        let faction_relations = match character_type {
            CharacterType::Player => FactionRelations::default_for_player(),
            CharacterType::Enemy => FactionRelations::default_for_enemy(),
            _ => FactionRelations::default(),
        };
        Self {
            id,
            character_type,
            location,
            attributes: HashMap::new(),
            interactions: Vec::new(),
            innate_abilities: Vec::new(),
            config_id: None,
            name: String::new(),
            description: Description::default(),
            ai: None,
            battle_ai: BattleAiConfig::default(),
            factions,
            faction_relations,
            active_effects: Vec::new(),
            inventory: Inventory::default(),
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
    fn character_new_stores_location() {
        let loc = test_location();
        let character = Character::new(1, CharacterType::Player, loc);
        assert_eq!(character.location.world_id, "w1");
        assert_eq!(character.location.dungeon_id, "d1");
        assert_eq!(character.location.room_id, "r1");
    }

    #[test]
    fn character_implements_entity_trait() {
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character.name = "Aragorn".to_string();
        character.description = Description::new(Some("A ranger.".to_string()));
        assert_eq!(character.id(), character.id);
        assert_eq!(character.name(), "Aragorn");
        assert_eq!(character.description().text.as_deref(), Some("A ranger."));
        assert_eq!(character.location().world_id, "w1");
    }

    #[test]
    fn player_character_has_player_faction() {
        let character = Character::new(1, CharacterType::Player, test_location());
        assert!(character.factions.contains("player"));
        assert_eq!(character.factions.len(), 1);
    }

    #[test]
    fn enemy_character_has_enemy_faction() {
        let character = Character::new(2, CharacterType::Enemy, test_location());
        assert!(character.factions.contains("enemy"));
        assert_eq!(character.factions.len(), 1);
    }

    #[test]
    fn non_player_character_has_empty_factions() {
        let character = Character::new(2, CharacterType::Character, test_location());
        assert!(character.factions.is_empty());
    }

    #[test]
    fn player_character_has_default_faction_relations() {
        let character = Character::new(1, CharacterType::Player, test_location());
        assert_eq!(
            character.faction_relations.player_relation(),
            &FactionRelation::Friendly
        );
        assert_eq!(
            character.faction_relations.enemy_relation(),
            &FactionRelation::Hostile
        );
    }

    #[test]
    fn enemy_character_has_default_faction_relations() {
        let character = Character::new(2, CharacterType::Enemy, test_location());
        assert_eq!(
            character.faction_relations.player_relation(),
            &FactionRelation::Hostile
        );
        assert_eq!(
            character.faction_relations.enemy_relation(),
            &FactionRelation::NonInteractive
        );
    }

    #[test]
    fn character_has_empty_faction_relations() {
        let character = Character::new(3, CharacterType::Character, test_location());
        assert!(character.faction_relations.factions.is_empty());
    }
}
