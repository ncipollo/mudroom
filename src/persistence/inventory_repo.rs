use sqlx::SqlitePool;

use crate::game::component::Item;
use crate::persistence::error::PersistenceError;

pub async fn ensure_exists(pool: &SqlitePool, character_id: i64) -> Result<(), PersistenceError> {
    sqlx::query("INSERT OR IGNORE INTO inventories (character_id) VALUES (?)")
        .bind(character_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_character_equipped_items(
    pool: &SqlitePool,
    character_id: i64,
    item_definition_ids: &[&str],
) -> Result<(), PersistenceError> {
    ensure_exists(pool, character_id).await?;
    sqlx::query("DELETE FROM inventory_items WHERE character_id = ? AND equipped = 1")
        .bind(character_id)
        .execute(pool)
        .await?;
    for item_definition_id in item_definition_ids {
        sqlx::query(
            "INSERT INTO inventory_items (character_id, item_definition_id, equipped) VALUES (?, ?, 1)",
        )
        .bind(character_id)
        .bind(item_definition_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn find_equipped_by_character(
    pool: &SqlitePool,
    character_id: i64,
) -> Result<Vec<Item>, PersistenceError> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, item_definition_id FROM inventory_items WHERE character_id = ? AND equipped = 1",
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

        set_character_equipped_items(db.pool(), character_id, &["leather_vest"])
            .await
            .unwrap();

        let items = find_equipped_by_character(db.pool(), character_id)
            .await
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_definition_id, "leather_vest");
    }

    #[tokio::test]
    async fn set_character_equipped_items_replaces_existing() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        set_character_equipped_items(db.pool(), character_id, &["leather_vest"])
            .await
            .unwrap();
        set_character_equipped_items(db.pool(), character_id, &[])
            .await
            .unwrap();

        let items = find_equipped_by_character(db.pool(), character_id)
            .await
            .unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn set_character_equipped_items_is_idempotent() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        set_character_equipped_items(db.pool(), character_id, &["leather_vest"])
            .await
            .unwrap();
        set_character_equipped_items(db.pool(), character_id, &["leather_vest"])
            .await
            .unwrap();

        let items = find_equipped_by_character(db.pool(), character_id)
            .await
            .unwrap();
        assert_eq!(items.len(), 1);
    }

    #[tokio::test]
    async fn cascade_delete_on_character_delete() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        set_character_equipped_items(db.pool(), character_id, &["leather_vest"])
            .await
            .unwrap();

        character_repo::delete(db.pool(), character_id)
            .await
            .unwrap();

        let items = find_equipped_by_character(db.pool(), character_id)
            .await
            .unwrap();
        assert!(items.is_empty());
    }
}
