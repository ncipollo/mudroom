use sqlx::SqlitePool;

use crate::game::component::Location;
use crate::game::component::description::Description;
use crate::game::entity::world_loot::WorldLoot;
use crate::persistence::error::PersistenceError;

type WorldLootRow = (i64, String, String, String, String);

pub async fn insert(pool: &SqlitePool, loot: &WorldLoot) -> Result<i64, PersistenceError> {
    let result = sqlx::query(
        "INSERT INTO world_loot (item_definition_id, world_id, dungeon_id, room_id) VALUES (?, ?, ?, ?)",
    )
    .bind(&loot.item_definition_id)
    .bind(&loot.location.world_id)
    .bind(&loot.location.dungeon_id)
    .bind(&loot.location.room_id)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn find_by_location(
    pool: &SqlitePool,
    location: &Location,
) -> Result<Vec<WorldLoot>, PersistenceError> {
    let rows: Vec<WorldLootRow> = sqlx::query_as(
        "SELECT id, item_definition_id, world_id, dungeon_id, room_id FROM world_loot \
         WHERE world_id = ? AND dungeon_id = ? AND room_id = ?",
    )
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(id, item_definition_id, world_id, dungeon_id, room_id)| WorldLoot {
                id,
                item_definition_id,
                location: Location {
                    world_id,
                    dungeon_id,
                    room_id,
                },
                name: String::new(),
                description: Description::default(),
            },
        )
        .collect())
}

pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM world_loot WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::{EquippedBonuses, ItemDefinition, ItemUseType};
    use crate::game::{Dungeon, Room, World};
    use crate::persistence::database::Database;
    use crate::persistence::{dungeon_repo, item_repo, room_repo, world_repo};

    fn test_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    async fn setup(db: &Database) {
        let world = World::new("w1".to_string());
        world_repo::insert(db.pool(), &world).await.unwrap();
        let dungeon = Dungeon::new("d1".to_string());
        dungeon_repo::insert(db.pool(), &dungeon, "w1")
            .await
            .unwrap();
        let room = Room::new("r1".to_string(), Description::new(None));
        room_repo::insert(db.pool(), &room, "d1").await.unwrap();

        let def = ItemDefinition {
            id: "health_tonic".to_string(),
            name: "Health Tonic".to_string(),
            description: Description::new(Some("A restorative brew.".to_string())),
            use_type: ItemUseType::Used,
            item_type: "medicine".to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects: vec![],
        };
        item_repo::upsert_definition(db.pool(), &def).await.unwrap();
    }

    #[tokio::test]
    async fn insert_and_find_by_location() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let loot = WorldLoot::new(
            0,
            "health_tonic".to_string(),
            test_location(),
            "Health Tonic".to_string(),
            Description::new(Some("A restorative brew.".to_string())),
        );
        let id = insert(db.pool(), &loot).await.unwrap();

        let found = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, id);
        assert_eq!(found[0].item_definition_id, "health_tonic");
    }

    #[tokio::test]
    async fn delete_removes_loot() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let loot = WorldLoot::new(
            0,
            "health_tonic".to_string(),
            test_location(),
            "Health Tonic".to_string(),
            Description::new(Some("A restorative brew.".to_string())),
        );
        let id = insert(db.pool(), &loot).await.unwrap();
        delete(db.pool(), id).await.unwrap();

        let found = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn find_by_location_is_empty_for_other_rooms() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let loot = WorldLoot::new(
            0,
            "health_tonic".to_string(),
            test_location(),
            "Health Tonic".to_string(),
            Description::new(Some("A restorative brew.".to_string())),
        );
        insert(db.pool(), &loot).await.unwrap();

        let other_location = Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r2".to_string(),
        };
        let found = find_by_location(db.pool(), &other_location).await.unwrap();
        assert!(found.is_empty());
    }
}
