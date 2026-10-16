use sqlx::SqlitePool;
use thiserror::Error;

use crate::game::GameState;
use crate::game::component::Location;
use crate::game::config::character_config::CharacterTypeConfig;
use crate::game::entity::character::{Character, CharacterType};
use crate::persistence::{PersistenceError, character_repo};

#[derive(Debug, Error)]
pub enum EntitySpawnError {
    #[error("unknown character config id: {0}")]
    UnknownConfigId(String),
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
}

/// Creates a new character from `config_id` at `location` outside the normal map-sync cycle,
/// persisting it and inserting it into `active_characters` so it's immediately live. Returns
/// the new character's id.
pub async fn spawn(
    game_state: &GameState,
    pool: &SqlitePool,
    config_id: &str,
    location: Location,
) -> Result<i64, EntitySpawnError> {
    let config = game_state
        .character_configs
        .get(config_id)
        .ok_or_else(|| EntitySpawnError::UnknownConfigId(config_id.to_string()))?;

    let character_type = match config.entity_type {
        CharacterTypeConfig::Character => CharacterType::Character,
        CharacterTypeConfig::Enemy => CharacterType::Enemy,
    };

    let mut character = Character::new(0, character_type, location);
    character.config_id = Some(config_id.to_string());
    character.name = config.name.clone().unwrap_or_default();
    character.description = config.description.clone();

    let id = character_repo::insert(pool, &character).await?;
    character.id = id;

    game_state
        .active_characters
        .write()
        .await
        .insert(id, character);

    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::description::Description;
    use crate::game::config::character_config::{CharacterConfig, CharacterTypeConfig};
    use crate::game::{Dungeon, Room, World};
    use crate::persistence::Database;
    use crate::persistence::{dungeon_repo, room_repo, world_repo};

    fn test_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    async fn seed_location(db: &Database) {
        let world = World::new("w1".to_string());
        world_repo::insert(db.pool(), &world).await.unwrap();
        let dungeon = Dungeon::new("d1".to_string());
        dungeon_repo::insert(db.pool(), &dungeon, "w1")
            .await
            .unwrap();
        let room = Room::new("r1".to_string(), Description::new(None));
        room_repo::insert(db.pool(), &room, "d1").await.unwrap();
    }

    fn zombie_config() -> CharacterConfig {
        CharacterConfig {
            id: Some("entities/zombie".to_string()),
            name: Some("Zombie".to_string()),
            entity_type: CharacterTypeConfig::Enemy,
            description: Description::new(Some("A shambling corpse.".to_string())),
            persona: None,
            attributes: Vec::new(),
            entity_effects: Vec::new(),
            innate_abilities: Vec::new(),
            factions: Vec::new(),
            faction_relations: None,
            battle_ai: Default::default(),
        }
    }

    fn game_state_with_zombie_config() -> GameState {
        let mut game_state = GameState::load(None).unwrap();
        game_state
            .character_configs
            .insert("entities/zombie".to_string(), zombie_config());
        game_state
    }

    #[tokio::test]
    async fn spawn_persists_the_new_character() {
        let db = Database::connect_in_memory().await.unwrap();
        seed_location(&db).await;
        let game_state = game_state_with_zombie_config();

        let id = spawn(&game_state, db.pool(), "entities/zombie", test_location())
            .await
            .unwrap();

        let persisted = character_repo::find_by_id(db.pool(), id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(persisted.config_id.as_deref(), Some("entities/zombie"));
        assert_eq!(persisted.name, "Zombie");
        assert!(matches!(persisted.character_type, CharacterType::Enemy));
        assert_eq!(persisted.location.room_id, "r1");
    }

    #[tokio::test]
    async fn spawn_inserts_into_active_characters() {
        let db = Database::connect_in_memory().await.unwrap();
        seed_location(&db).await;
        let game_state = game_state_with_zombie_config();

        let id = spawn(&game_state, db.pool(), "entities/zombie", test_location())
            .await
            .unwrap();

        let active = game_state.active_characters.read().await;
        let character = active.get(&id).expect("spawned character should be live");
        assert_eq!(character.config_id.as_deref(), Some("entities/zombie"));
        assert_eq!(character.id, id);
    }

    #[tokio::test]
    async fn spawn_sets_description_from_config() {
        let db = Database::connect_in_memory().await.unwrap();
        seed_location(&db).await;
        let game_state = game_state_with_zombie_config();

        let id = spawn(&game_state, db.pool(), "entities/zombie", test_location())
            .await
            .unwrap();

        let active = game_state.active_characters.read().await;
        let character = active.get(&id).unwrap();
        assert_eq!(
            character.description.text.as_deref(),
            Some("A shambling corpse.")
        );
    }

    #[tokio::test]
    async fn spawn_rejects_unknown_config_id() {
        let db = Database::connect_in_memory().await.unwrap();
        let game_state = GameState::load(None).unwrap();

        let result = spawn(&game_state, db.pool(), "entities/nope", test_location()).await;

        assert!(
            matches!(result, Err(EntitySpawnError::UnknownConfigId(id)) if id == "entities/nope")
        );
    }

    #[tokio::test]
    async fn spawn_twice_creates_two_distinct_characters() {
        let db = Database::connect_in_memory().await.unwrap();
        seed_location(&db).await;
        let game_state = game_state_with_zombie_config();

        let first = spawn(&game_state, db.pool(), "entities/zombie", test_location())
            .await
            .unwrap();
        let second = spawn(&game_state, db.pool(), "entities/zombie", test_location())
            .await
            .unwrap();

        assert_ne!(first, second);
        let active = game_state.active_characters.read().await;
        assert_eq!(active.len(), 2);
    }
}
