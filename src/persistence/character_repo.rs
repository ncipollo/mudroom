use std::collections::{HashMap, HashSet};

use sqlx::SqlitePool;

use crate::game::component::Attribute;
use crate::game::component::description::Description;
use crate::game::config::{BattleAiConfig, BattleAiType};
use crate::game::{Character, CharacterType, Location};
use crate::persistence::error::PersistenceError;
use crate::persistence::{ability_repo, faction_relations_repo, faction_repo, inventory_repo};

type CharacterRow = (
    i64,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
);

pub async fn insert(pool: &SqlitePool, character: &Character) -> Result<i64, PersistenceError> {
    let character_type = character_type_to_str(&character.character_type);
    let attributes_json = serde_json::to_string(&character.attributes)?;
    let result = sqlx::query(
        "INSERT INTO characters (character_type, world_id, dungeon_id, room_id, config_id, attributes, description, name) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(character_type)
    .bind(&character.location.world_id)
    .bind(&character.location.dungeon_id)
    .bind(&character.location.room_id)
    .bind(&character.config_id)
    .bind(attributes_json)
    .bind(&character.description.text)
    .bind(&character.name)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

/// Insert a config character if no character with the same config_id + original location exists.
/// Returns `(character_id, is_new)` — id of the existing or newly inserted character, and whether it
/// was just created. Current location is preserved on conflict (character may have moved).
pub async fn insert_config_character_if_missing(
    pool: &SqlitePool,
    character_type: &CharacterType,
    location: &Location,
    config_id: &str,
    description: Option<&str>,
    name: &str,
) -> Result<(i64, bool), PersistenceError> {
    let character_type_str = character_type_to_str(character_type);

    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM characters
         WHERE config_id = ? AND original_world_id = ? AND original_dungeon_id = ? AND original_room_id = ?",
    )
    .bind(config_id)
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .fetch_optional(pool)
    .await?;

    if let Some((id,)) = existing {
        sqlx::query(
            "UPDATE characters SET character_type = ?, description = ?, name = ? WHERE id = ?",
        )
        .bind(character_type_str)
        .bind(description)
        .bind(name)
        .bind(id)
        .execute(pool)
        .await?;
        return Ok((id, false));
    }

    let result = sqlx::query(
        "INSERT INTO characters
             (character_type, world_id, dungeon_id, room_id, config_id,
              original_world_id, original_dungeon_id, original_room_id, description, name)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(character_type_str)
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .bind(config_id)
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .bind(description)
    .bind(name)
    .execute(pool)
    .await?;

    Ok((result.last_insert_rowid(), true))
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Character>, PersistenceError> {
    let row: Option<CharacterRow> = sqlx::query_as(
        "SELECT id, character_type, world_id, dungeon_id, room_id, config_id, attributes, description, battle_ai_type, name FROM characters WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        let mut character = build_character(row);
        load_faction_data(pool, &mut character).await?;
        load_ability_data(pool, &mut character).await?;
        load_inventory_data(pool, &mut character).await?;
        Ok(Some(character))
    } else {
        Ok(None)
    }
}

pub async fn find_by_location(
    pool: &SqlitePool,
    location: &Location,
) -> Result<Vec<Character>, PersistenceError> {
    let rows: Vec<CharacterRow> = sqlx::query_as(
        "SELECT id, character_type, world_id, dungeon_id, room_id, config_id, attributes, description, battle_ai_type, name FROM characters WHERE world_id = ? AND dungeon_id = ? AND room_id = ?",
    )
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .fetch_all(pool)
    .await?;

    let mut characters = Vec::new();
    for row in rows {
        let mut character = build_character(row);
        load_faction_data(pool, &mut character).await?;
        load_ability_data(pool, &mut character).await?;
        load_inventory_data(pool, &mut character).await?;
        characters.push(character);
    }
    Ok(characters)
}

pub async fn find_config_characters_by_dungeon(
    pool: &SqlitePool,
    world_id: &str,
    dungeon_id: &str,
) -> Result<Vec<Character>, PersistenceError> {
    let rows: Vec<CharacterRow> = sqlx::query_as(
        "SELECT id, character_type, world_id, dungeon_id, room_id, config_id, attributes, description, battle_ai_type, name FROM characters WHERE config_id IS NOT NULL AND world_id = ? AND dungeon_id = ?",
    )
    .bind(world_id)
    .bind(dungeon_id)
    .fetch_all(pool)
    .await?;

    let mut characters = Vec::new();
    for row in rows {
        let mut character = build_character(row);
        load_faction_data(pool, &mut character).await?;
        load_ability_data(pool, &mut character).await?;
        load_inventory_data(pool, &mut character).await?;
        characters.push(character);
    }
    Ok(characters)
}

fn build_character(
    (
        id,
        et,
        world_id,
        dungeon_id,
        room_id,
        config_id,
        attrs_json,
        description,
        battle_ai_type,
        name,
    ): CharacterRow,
) -> Character {
    let attributes = attrs_json
        .and_then(|json| match serde_json::from_str(&json) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::warn!("Failed to deserialize attributes for character {id}: {e}");
                None
            }
        })
        .unwrap_or_default();
    let mut character = Character::new(
        id,
        character_type_from_str(&et),
        Location {
            world_id,
            dungeon_id,
            room_id,
        },
    );
    character.config_id = config_id;
    character.name = name;
    character.attributes = attributes;
    character.description = Description::new(description);
    character.battle_ai = BattleAiConfig {
        ai_type: battle_ai_type_from_str(&battle_ai_type),
    };
    character
}

pub async fn update_battle_ai_type(
    pool: &SqlitePool,
    character_id: i64,
    ai_type: &BattleAiType,
) -> Result<(), PersistenceError> {
    sqlx::query("UPDATE characters SET battle_ai_type = ? WHERE id = ?")
        .bind(battle_ai_type_to_str(ai_type))
        .bind(character_id)
        .execute(pool)
        .await?;
    Ok(())
}

fn battle_ai_type_to_str(ai_type: &BattleAiType) -> &'static str {
    match ai_type {
        BattleAiType::None => "none",
        BattleAiType::SimpleRandom => "simple_random",
    }
}

fn battle_ai_type_from_str(s: &str) -> BattleAiType {
    match s {
        "simple_random" => BattleAiType::SimpleRandom,
        _ => BattleAiType::None,
    }
}

async fn load_faction_data(
    pool: &SqlitePool,
    character: &mut Character,
) -> Result<(), PersistenceError> {
    let db_factions: HashSet<String> = faction_repo::find_by_character(pool, character.id)
        .await?
        .into_iter()
        .map(|f| f.id)
        .collect();
    if !db_factions.is_empty() {
        character.factions = db_factions;
    }
    let db_relations = faction_relations_repo::find_by_character(pool, character.id).await?;
    if !db_relations.factions.is_empty() {
        character.faction_relations = db_relations;
    }
    Ok(())
}

async fn load_ability_data(
    pool: &SqlitePool,
    character: &mut Character,
) -> Result<(), PersistenceError> {
    let abilities = ability_repo::find_by_character(pool, character.id).await?;
    if !abilities.is_empty() {
        character.innate_abilities = abilities;
    }
    Ok(())
}

async fn load_inventory_data(
    pool: &SqlitePool,
    character: &mut Character,
) -> Result<(), PersistenceError> {
    let items = inventory_repo::find_equipped_by_character(pool, character.id).await?;
    if !items.is_empty() {
        character.inventory.equipped_items = items;
    }
    Ok(())
}

pub async fn update_attributes(
    pool: &SqlitePool,
    character_id: i64,
    attributes: &HashMap<String, Attribute>,
) -> Result<(), PersistenceError> {
    let json = serde_json::to_string(attributes)?;
    sqlx::query("UPDATE characters SET attributes = ? WHERE id = ?")
        .bind(json)
        .bind(character_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_location(
    pool: &SqlitePool,
    character_id: i64,
    location: &Location,
) -> Result<(), PersistenceError> {
    sqlx::query("UPDATE characters SET world_id = ?, dungeon_id = ?, room_id = ? WHERE id = ?")
        .bind(&location.world_id)
        .bind(&location.dungeon_id)
        .bind(&location.room_id)
        .bind(character_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM characters WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_by_room(pool: &SqlitePool, room_id: &str) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM characters WHERE room_id = ?")
        .bind(room_id)
        .execute(pool)
        .await?;
    Ok(())
}

fn character_type_to_str(et: &CharacterType) -> &'static str {
    match et {
        CharacterType::Player => "player",
        CharacterType::Character => "character",
        CharacterType::Enemy => "enemy",
    }
}

fn character_type_from_str(s: &str) -> CharacterType {
    match s {
        "player" => CharacterType::Player,
        "enemy" => CharacterType::Enemy,
        _ => CharacterType::Character,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Room;
    use crate::game::{Dungeon, World};
    use crate::persistence::database::Database;
    use crate::persistence::{dungeon_repo, room_repo, world_repo};

    async fn setup(db: &Database) {
        let world = World::new("w1".to_string());
        world_repo::insert(db.pool(), &world).await.unwrap();
        let dungeon = Dungeon::new("d1".to_string());
        dungeon_repo::insert(db.pool(), &dungeon, "w1")
            .await
            .unwrap();
        let room = Room::new("r1".to_string(), Description::new(None));
        room_repo::insert(db.pool(), &room, "d1").await.unwrap();
    }

    fn test_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    #[tokio::test]
    async fn insert_and_find_by_id() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        let character = Character::new(0, CharacterType::Player, test_location());
        let id = insert(db.pool(), &character).await.unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.id, id);
        assert_eq!(found.location.world_id, "w1");
    }

    #[tokio::test]
    async fn find_by_id_returns_none_for_missing() {
        let db = Database::connect_in_memory().await.unwrap();
        let found = find_by_id(db.pool(), 999).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn find_by_location_returns_characters() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        insert(
            db.pool(),
            &Character::new(0, CharacterType::Player, test_location()),
        )
        .await
        .unwrap();
        insert(
            db.pool(),
            &Character::new(0, CharacterType::Character, test_location()),
        )
        .await
        .unwrap();

        let characters = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert_eq!(characters.len(), 2);
    }

    #[tokio::test]
    async fn update_location_changes_location() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        // Add a second room for the new location
        let room2 = Room::new("r2".to_string(), Description::new(None));
        room_repo::insert(db.pool(), &room2, "d1").await.unwrap();

        let character = Character::new(0, CharacterType::Player, test_location());
        let id = insert(db.pool(), &character).await.unwrap();

        let new_loc = Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r2".to_string(),
        };
        update_location(db.pool(), id, &new_loc).await.unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.location.room_id, "r2");
    }

    #[tokio::test]
    async fn delete_removes_character() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        let character = Character::new(0, CharacterType::Player, test_location());
        let id = insert(db.pool(), &character).await.unwrap();
        delete(db.pool(), id).await.unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn delete_by_room_removes_all_characters_in_room() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        insert(
            db.pool(),
            &Character::new(0, CharacterType::Player, test_location()),
        )
        .await
        .unwrap();
        insert(
            db.pool(),
            &Character::new(0, CharacterType::Character, test_location()),
        )
        .await
        .unwrap();

        delete_by_room(db.pool(), "r1").await.unwrap();

        let characters = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert!(characters.is_empty());
    }

    #[tokio::test]
    async fn find_config_characters_by_dungeon_returns_matching() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id, _) = insert_config_character_if_missing(
            db.pool(),
            &CharacterType::Character,
            &test_location(),
            "entities/innkeeper",
            None,
            "innkeeper",
        )
        .await
        .unwrap();

        // Player character (no config_id) should not be returned
        insert(
            db.pool(),
            &Character::new(0, CharacterType::Player, test_location()),
        )
        .await
        .unwrap();

        let found = find_config_characters_by_dungeon(db.pool(), "w1", "d1")
            .await
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, id);

        // Different dungeon returns nothing
        let other = find_config_characters_by_dungeon(db.pool(), "w1", "d2")
            .await
            .unwrap();
        assert!(other.is_empty());
    }

    #[tokio::test]
    async fn insert_and_find_preserves_name() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let mut character = Character::new(0, CharacterType::Player, test_location());
        character.name = "Aragorn".to_string();
        let id = insert(db.pool(), &character).await.unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.name, "Aragorn");
    }

    #[tokio::test]
    async fn insert_config_character_if_missing_stores_name() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id, _) = insert_config_character_if_missing(
            db.pool(),
            &CharacterType::Character,
            &test_location(),
            "entities/innkeeper",
            None,
            "innkeeper",
        )
        .await
        .unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.name, "innkeeper");
    }

    #[tokio::test]
    async fn insert_config_character_if_missing_updates_name_on_conflict() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id, _) = insert_config_character_if_missing(
            db.pool(),
            &CharacterType::Character,
            &test_location(),
            "entities/innkeeper",
            None,
            "old name",
        )
        .await
        .unwrap();

        insert_config_character_if_missing(
            db.pool(),
            &CharacterType::Character,
            &test_location(),
            "entities/innkeeper",
            None,
            "innkeeper",
        )
        .await
        .unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.name, "innkeeper");
    }

    #[tokio::test]
    async fn insert_config_character_if_missing_stores_description() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id, _) = insert_config_character_if_missing(
            db.pool(),
            &CharacterType::Character,
            &test_location(),
            "entities/innkeeper",
            Some("A friendly innkeeper."),
            "innkeeper",
        )
        .await
        .unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(
            found.description.text.as_deref(),
            Some("A friendly innkeeper.")
        );
    }

    #[tokio::test]
    async fn insert_config_character_if_missing_updates_description_on_conflict() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id, _) = insert_config_character_if_missing(
            db.pool(),
            &CharacterType::Character,
            &test_location(),
            "entities/innkeeper",
            Some("Old description."),
            "innkeeper",
        )
        .await
        .unwrap();

        insert_config_character_if_missing(
            db.pool(),
            &CharacterType::Character,
            &test_location(),
            "entities/innkeeper",
            Some("New description."),
            "innkeeper",
        )
        .await
        .unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.description.text.as_deref(), Some("New description."));
    }

    #[tokio::test]
    async fn insert_config_character_if_missing_inserts_new() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id, is_new) = insert_config_character_if_missing(
            db.pool(),
            &CharacterType::Character,
            &test_location(),
            "entities/innkeeper",
            None,
            "innkeeper",
        )
        .await
        .unwrap();
        assert!(id > 0);
        assert!(is_new);

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.config_id.as_deref(), Some("entities/innkeeper"));
    }

    #[tokio::test]
    async fn insert_config_character_if_missing_returns_existing_id() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id1, is_new1) = insert_config_character_if_missing(
            db.pool(),
            &CharacterType::Character,
            &test_location(),
            "entities/innkeeper",
            None,
            "innkeeper",
        )
        .await
        .unwrap();
        let (id2, is_new2) = insert_config_character_if_missing(
            db.pool(),
            &CharacterType::Character,
            &test_location(),
            "entities/innkeeper",
            None,
            "innkeeper",
        )
        .await
        .unwrap();
        assert_eq!(id1, id2);
        assert!(is_new1);
        assert!(!is_new2);

        let characters = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert_eq!(characters.len(), 1);
    }

    #[tokio::test]
    async fn insert_and_find_preserves_attributes() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let mut character = Character::new(0, CharacterType::Character, test_location());
        character.attributes.insert(
            "hp".to_string(),
            Attribute::new("hp".to_string(), 0, 100, 80),
        );
        character.attributes.insert(
            "mp".to_string(),
            Attribute::new("mp".to_string(), 0, 50, 50),
        );
        let id = insert(db.pool(), &character).await.unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.attributes.len(), 2);
        assert_eq!(found.attributes["hp"].current_value, 80);
        assert_eq!(found.attributes["mp"].max_value, 50);
    }

    #[tokio::test]
    async fn find_by_id_with_corrupt_attributes_returns_empty() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let character = Character::new(0, CharacterType::Character, test_location());
        let id = insert(db.pool(), &character).await.unwrap();

        // Corrupt the attributes column
        sqlx::query("UPDATE characters SET attributes = ? WHERE id = ?")
            .bind("not valid json{{{")
            .bind(id)
            .execute(db.pool())
            .await
            .unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert!(found.attributes.is_empty());
    }

    #[tokio::test]
    async fn find_by_id_loads_factions_and_faction_relations() {
        use crate::game::component::Faction;
        use crate::game::component::faction_relations::FactionRelation;
        use crate::persistence::{faction_relations_repo, faction_repo};

        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let character = Character::new(0, CharacterType::Enemy, test_location());
        let id = insert(db.pool(), &character).await.unwrap();

        faction_repo::upsert(
            db.pool(),
            &Faction {
                id: "enemy".to_string(),
                name: "Enemy".to_string(),
                description: "Hostile creatures.".to_string(),
            },
        )
        .await
        .unwrap();
        faction_repo::set_character_factions(db.pool(), id, &["enemy".to_string()])
            .await
            .unwrap();

        use crate::game::component::FactionRelations;
        faction_relations_repo::set_character_relations(
            db.pool(),
            id,
            &FactionRelations::default_for_enemy(),
        )
        .await
        .unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert!(found.factions.contains("enemy"));
        assert_eq!(
            found.faction_relations.player_relation(),
            &FactionRelation::Hostile
        );
    }

    #[tokio::test]
    async fn find_by_id_uses_character_defaults_when_no_faction_data() {
        use crate::game::component::faction_relations::FactionRelation;

        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let character = Character::new(0, CharacterType::Enemy, test_location());
        let id = insert(db.pool(), &character).await.unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert!(found.factions.contains("enemy"));
        assert_eq!(
            found.faction_relations.player_relation(),
            &FactionRelation::Hostile
        );
    }

    #[tokio::test]
    async fn update_attributes_persists_changes() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let character = Character::new(0, CharacterType::Character, test_location());
        let id = insert(db.pool(), &character).await.unwrap();

        let mut attrs = HashMap::new();
        attrs.insert(
            "str".to_string(),
            Attribute::new("str".to_string(), 1, 20, 15),
        );
        update_attributes(db.pool(), id, &attrs).await.unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.attributes.len(), 1);
        assert_eq!(found.attributes["str"].current_value, 15);
    }

    #[tokio::test]
    async fn insert_and_find_preserves_default_battle_ai_type() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let character = Character::new(0, CharacterType::Enemy, test_location());
        let id = insert(db.pool(), &character).await.unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.battle_ai.ai_type, BattleAiType::None);
    }

    #[tokio::test]
    async fn update_battle_ai_type_persists_simple_random() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let character = Character::new(0, CharacterType::Enemy, test_location());
        let id = insert(db.pool(), &character).await.unwrap();

        update_battle_ai_type(db.pool(), id, &BattleAiType::SimpleRandom)
            .await
            .unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.battle_ai.ai_type, BattleAiType::SimpleRandom);
    }

    #[tokio::test]
    async fn update_battle_ai_type_can_reset_to_none() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let character = Character::new(0, CharacterType::Enemy, test_location());
        let id = insert(db.pool(), &character).await.unwrap();

        update_battle_ai_type(db.pool(), id, &BattleAiType::SimpleRandom)
            .await
            .unwrap();
        update_battle_ai_type(db.pool(), id, &BattleAiType::None)
            .await
            .unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.battle_ai.ai_type, BattleAiType::None);
    }

    #[tokio::test]
    async fn find_by_id_loads_innate_abilities() {
        use crate::game::component::{Ability, AbilityRole};
        use crate::game::engagement::EngagementType;
        use crate::persistence::ability_repo;

        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let character = Character::new(0, CharacterType::Enemy, test_location());
        let id = insert(db.pool(), &character).await.unwrap();

        let ability = Ability {
            id: "strike".to_string(),
            name: "Strike".to_string(),
            description: Description::default(),
            effects: vec![],
            costs: vec![],
            modifiers: vec![],
            engagement_types: vec![EngagementType::Battle],
            role: AbilityRole::Attack,
            targets: vec![],
            action_text: None,
        };
        ability_repo::upsert(db.pool(), &ability).await.unwrap();
        ability_repo::set_character_abilities(db.pool(), id, &["strike"])
            .await
            .unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.innate_abilities.len(), 1);
        assert_eq!(found.innate_abilities[0].id, "strike");
    }

    #[tokio::test]
    async fn find_by_id_loads_equipped_inventory() {
        use crate::game::component::{EquippedBonuses, ItemDefinition, ItemUseType};
        use crate::persistence::{inventory_repo, item_repo};

        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let character = Character::new(0, CharacterType::Player, test_location());
        let id = insert(db.pool(), &character).await.unwrap();

        let def = ItemDefinition {
            id: "leather_vest".to_string(),
            name: "Leather Vest".to_string(),
            description: Description::default(),
            use_type: ItemUseType::Passive,
            item_type: "armor".to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects: vec![],
        };
        item_repo::upsert_definition(db.pool(), &def).await.unwrap();
        inventory_repo::set_character_equipped_items(db.pool(), id, &["leather_vest"])
            .await
            .unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.inventory.equipped_items.len(), 1);
        assert_eq!(
            found.inventory.equipped_items[0].item_definition_id,
            "leather_vest"
        );
    }
}
