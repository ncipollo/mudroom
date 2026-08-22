use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::game::component::Item;
use crate::persistence::error::PersistenceError;

pub async fn ensure_exists(
    pool: &SqlitePool,
    character_id: i64,
    inventory_type: &str,
) -> Result<(), PersistenceError> {
    sqlx::query("INSERT OR IGNORE INTO inventories (character_id, inventory_type) VALUES (?, ?)")
        .bind(character_id)
        .bind(inventory_type)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_inventory_type(
    pool: &SqlitePool,
    character_id: i64,
) -> Result<Option<String>, PersistenceError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT inventory_type FROM inventories WHERE character_id = ?")
            .bind(character_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(inventory_type,)| inventory_type))
}

pub async fn set_character_equipment(
    pool: &SqlitePool,
    character_id: i64,
    equipment: &[(&str, &str)],
) -> Result<(), PersistenceError> {
    ensure_exists(pool, character_id, "inventory").await?;
    sqlx::query("DELETE FROM inventory_items WHERE character_id = ? AND equipped = 1")
        .bind(character_id)
        .execute(pool)
        .await?;
    for (slot_name, item_definition_id) in equipment {
        sqlx::query(
            "INSERT INTO inventory_items (character_id, item_definition_id, equipped, slot_name) VALUES (?, ?, 1, ?)",
        )
        .bind(character_id)
        .bind(item_definition_id)
        .bind(slot_name)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn find_equipped_by_character(
    pool: &SqlitePool,
    character_id: i64,
) -> Result<HashMap<String, Item>, PersistenceError> {
    let rows: Vec<(i64, String, Option<String>)> = sqlx::query_as(
        "SELECT id, item_definition_id, slot_name FROM inventory_items WHERE character_id = ? AND equipped = 1",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|(id, item_definition_id, slot_name)| {
            slot_name.map(|slot_name| {
                (
                    slot_name,
                    Item {
                        id,
                        item_definition_id,
                    },
                )
            })
        })
        .collect())
}

pub async fn set_character_bag_items(
    pool: &SqlitePool,
    character_id: i64,
    item_definition_ids: &[&str],
) -> Result<(), PersistenceError> {
    ensure_exists(pool, character_id, "inventory").await?;
    sqlx::query("DELETE FROM inventory_items WHERE character_id = ? AND equipped = 0")
        .bind(character_id)
        .execute(pool)
        .await?;
    for item_definition_id in item_definition_ids {
        sqlx::query(
            "INSERT INTO inventory_items (character_id, item_definition_id, equipped) VALUES (?, ?, 0)",
        )
        .bind(character_id)
        .bind(item_definition_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn add_bag_item(
    pool: &SqlitePool,
    character_id: i64,
    item_definition_id: &str,
) -> Result<i64, PersistenceError> {
    ensure_exists(pool, character_id, "inventory").await?;
    let result = sqlx::query(
        "INSERT INTO inventory_items (character_id, item_definition_id, equipped) VALUES (?, ?, 0)",
    )
    .bind(character_id)
    .bind(item_definition_id)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn find_bag_by_character(
    pool: &SqlitePool,
    character_id: i64,
) -> Result<Vec<Item>, PersistenceError> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, item_definition_id FROM inventory_items WHERE character_id = ? AND equipped = 0",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, item_definition_id)| Item {
            id,
            item_definition_id,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::description::Description;
    use crate::game::component::{EquippedBonuses, ItemDefinition, ItemUseType};
    use crate::game::{Character, CharacterType, Dungeon, Location, Room, World};
    use crate::persistence::database::Database;
    use crate::persistence::{character_repo, dungeon_repo, item_repo, room_repo, world_repo};

    async fn setup(db: &Database) -> i64 {
        let world = World::new("w1".to_string());
        world_repo::insert(db.pool(), &world).await.unwrap();
        let dungeon = Dungeon::new("d1".to_string());
        dungeon_repo::insert(db.pool(), &dungeon, "w1")
            .await
            .unwrap();
        let room = Room::new("r1".to_string(), Description::new(None));
        room_repo::insert(db.pool(), &room, "d1").await.unwrap();
        let loc = Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        };
        let character = Character::new(0, CharacterType::Player, loc);
        let character_id = character_repo::insert(db.pool(), &character).await.unwrap();

        let def = ItemDefinition {
            id: "leather_vest".to_string(),
            name: "Leather Vest".to_string(),
            description: Description::new(Some("A simple vest.".to_string())),
            use_type: ItemUseType::Passive,
            item_type: "armor".to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects: vec![],
        };
        item_repo::upsert_definition(db.pool(), &def).await.unwrap();

        character_id
    }

    #[tokio::test]
    async fn set_and_find_equipped_items() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        set_character_equipment(db.pool(), character_id, &[("armor", "leather_vest")])
            .await
            .unwrap();

        let equipped = find_equipped_by_character(db.pool(), character_id)
            .await
            .unwrap();
        assert_eq!(equipped.len(), 1);
        assert_eq!(equipped["armor"].item_definition_id, "leather_vest");
    }

    #[tokio::test]
    async fn set_character_equipment_replaces_existing() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        set_character_equipment(db.pool(), character_id, &[("armor", "leather_vest")])
            .await
            .unwrap();
        set_character_equipment(db.pool(), character_id, &[])
            .await
            .unwrap();

        let equipped = find_equipped_by_character(db.pool(), character_id)
            .await
            .unwrap();
        assert!(equipped.is_empty());
    }

    #[tokio::test]
    async fn set_character_equipment_is_idempotent() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        set_character_equipment(db.pool(), character_id, &[("armor", "leather_vest")])
            .await
            .unwrap();
        set_character_equipment(db.pool(), character_id, &[("armor", "leather_vest")])
            .await
            .unwrap();

        let equipped = find_equipped_by_character(db.pool(), character_id)
            .await
            .unwrap();
        assert_eq!(equipped.len(), 1);
    }

    #[tokio::test]
    async fn set_and_find_bag_items() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        set_character_bag_items(db.pool(), character_id, &["leather_vest"])
            .await
            .unwrap();

        let bag = find_bag_by_character(db.pool(), character_id)
            .await
            .unwrap();
        assert_eq!(bag.len(), 1);
        assert_eq!(bag[0].item_definition_id, "leather_vest");
    }

    #[tokio::test]
    async fn add_bag_item_appends_without_replacing() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        set_character_bag_items(db.pool(), character_id, &["leather_vest"])
            .await
            .unwrap();
        add_bag_item(db.pool(), character_id, "leather_vest")
            .await
            .unwrap();

        let bag = find_bag_by_character(db.pool(), character_id)
            .await
            .unwrap();
        assert_eq!(bag.len(), 2);
    }

    #[tokio::test]
    async fn set_character_bag_items_replaces_existing() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        set_character_bag_items(db.pool(), character_id, &["leather_vest"])
            .await
            .unwrap();
        set_character_bag_items(db.pool(), character_id, &[])
            .await
            .unwrap();

        let bag = find_bag_by_character(db.pool(), character_id)
            .await
            .unwrap();
        assert!(bag.is_empty());
    }

    #[tokio::test]
    async fn bag_and_equipped_items_are_independent() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        set_character_bag_items(db.pool(), character_id, &["leather_vest"])
            .await
            .unwrap();
        set_character_equipment(db.pool(), character_id, &[("armor", "leather_vest")])
            .await
            .unwrap();

        let bag = find_bag_by_character(db.pool(), character_id)
            .await
            .unwrap();
        let equipped = find_equipped_by_character(db.pool(), character_id)
            .await
            .unwrap();
        assert_eq!(bag.len(), 1);
        assert_eq!(equipped.len(), 1);
    }

    #[tokio::test]
    async fn ensure_exists_sets_inventory_type() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        ensure_exists(db.pool(), character_id, "merchant_stall")
            .await
            .unwrap();

        let inventory_type = find_inventory_type(db.pool(), character_id).await.unwrap();
        assert_eq!(inventory_type.as_deref(), Some("merchant_stall"));
    }

    #[tokio::test]
    async fn find_inventory_type_returns_none_when_missing() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        let inventory_type = find_inventory_type(db.pool(), character_id).await.unwrap();
        assert!(inventory_type.is_none());
    }

    #[tokio::test]
    async fn cascade_delete_on_character_delete() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        set_character_equipment(db.pool(), character_id, &[("armor", "leather_vest")])
            .await
            .unwrap();
        set_character_bag_items(db.pool(), character_id, &["leather_vest"])
            .await
            .unwrap();

        character_repo::delete(db.pool(), character_id)
            .await
            .unwrap();

        let equipped = find_equipped_by_character(db.pool(), character_id)
            .await
            .unwrap();
        let bag = find_bag_by_character(db.pool(), character_id)
            .await
            .unwrap();
        assert!(equipped.is_empty());
        assert!(bag.is_empty());
    }
}
