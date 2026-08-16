use std::collections::{HashMap, HashSet};

use sqlx::SqlitePool;

use crate::game::component::Attribute;
use crate::game::component::description::Description;
use crate::game::config::{BattleAiConfig, BattleAiType, DEFAULT_INVENTORY_TYPE};
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
    let inventory_type = inventory_repo::find_inventory_type(pool, character.id).await?;
    character.inventory.inventory_type =
        inventory_type.unwrap_or_else(|| DEFAULT_INVENTORY_TYPE.to_string());
    character.inventory.equipment =
        inventory_repo::find_equipped_by_character(pool, character.id).await?;
    character.inventory.bag = inventory_repo::find_bag_by_character(pool, character.id).await?;
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
mod tests;
