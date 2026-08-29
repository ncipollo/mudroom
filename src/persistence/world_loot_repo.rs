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

/// Insert a config-seeded item if no world loot with the same item definition + original location
/// exists yet. Returns `(id, is_new)`. Unlike characters, item identity is already the definition
/// id, so no separate config_id is tracked — the dedupe key is `item_definition_id` + original
/// location. A row that already exists (taken or not) is left untouched: taking an item marks it
/// rather than deleting it (see `mark_taken`), so re-seeding never resurrects it here.
pub async fn insert_config_loot_if_missing(
    pool: &SqlitePool,
    location: &Location,
    item_definition_id: &str,
) -> Result<(i64, bool), PersistenceError> {
    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM world_loot
         WHERE item_definition_id = ? AND original_world_id = ? AND original_dungeon_id = ? AND original_room_id = ?",
    )
    .bind(item_definition_id)
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .fetch_optional(pool)
    .await?;

    if let Some((id,)) = existing {
        return Ok((id, false));
    }

    let result = sqlx::query(
        "INSERT INTO world_loot
             (item_definition_id, world_id, dungeon_id, room_id,
              original_world_id, original_dungeon_id, original_room_id)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(item_definition_id)
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .execute(pool)
    .await?;

    Ok((result.last_insert_rowid(), true))
}

pub async fn find_by_location(
    pool: &SqlitePool,
    location: &Location,
) -> Result<Vec<WorldLoot>, PersistenceError> {
    let rows: Vec<WorldLootRow> = sqlx::query_as(
        "SELECT id, item_definition_id, world_id, dungeon_id, room_id FROM world_loot \
         WHERE world_id = ? AND dungeon_id = ? AND room_id = ? AND taken = 0",
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

pub async fn delete_by_room(pool: &SqlitePool, room_id: &str) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM world_loot WHERE room_id = ?")
        .bind(room_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Marks a world loot row as taken rather than deleting it, so a later respawn (per the
/// configured `RespawnMode`) can bring it back without resurrecting rows re-seeding never touched.
pub async fn mark_taken(pool: &SqlitePool, id: i64) -> Result<(), PersistenceError> {
    sqlx::query("UPDATE world_loot SET taken = 1 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn respawn_room(pool: &SqlitePool, location: &Location) -> Result<(), PersistenceError> {
    sqlx::query(
        "UPDATE world_loot SET taken = 0 \
         WHERE taken = 1 AND original_world_id IS NOT NULL \
         AND world_id = ? AND dungeon_id = ? AND room_id = ?",
    )
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn respawn_dungeon(
    pool: &SqlitePool,
    world_id: &str,
    dungeon_id: &str,
) -> Result<(), PersistenceError> {
    sqlx::query(
        "UPDATE world_loot SET taken = 0 \
         WHERE taken = 1 AND original_world_id IS NOT NULL \
         AND world_id = ? AND dungeon_id = ?",
    )
    .bind(world_id)
    .bind(dungeon_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn respawn_all_config_seeded(pool: &SqlitePool) -> Result<(), PersistenceError> {
    sqlx::query(
        "UPDATE world_loot SET taken = 0 WHERE taken = 1 AND original_world_id IS NOT NULL",
    )
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

    fn other_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r2".to_string(),
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
        let room2 = Room::new("r2".to_string(), Description::new(None));
        room_repo::insert(db.pool(), &room2, "d1").await.unwrap();

        let def = ItemDefinition {
            id: "health_tonic".to_string(),
            name: "Health Tonic".to_string(),
            description: Description::new(Some("A restorative brew.".to_string())),
            use_type: ItemUseType::Used,
            item_type: "medicine".to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects: vec![],
            alternate_names: vec![],
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

        let found = find_by_location(db.pool(), &other_location())
            .await
            .unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn find_by_location_excludes_taken_loot() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id, is_new) =
            insert_config_loot_if_missing(db.pool(), &test_location(), "health_tonic")
                .await
                .unwrap();
        assert!(is_new);
        mark_taken(db.pool(), id).await.unwrap();

        let found = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn insert_config_loot_if_missing_is_idempotent() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id1, is_new1) =
            insert_config_loot_if_missing(db.pool(), &test_location(), "health_tonic")
                .await
                .unwrap();
        let (id2, is_new2) =
            insert_config_loot_if_missing(db.pool(), &test_location(), "health_tonic")
                .await
                .unwrap();

        assert!(is_new1);
        assert!(!is_new2);
        assert_eq!(id1, id2);
    }

    #[tokio::test]
    async fn insert_config_loot_if_missing_does_not_resurrect_taken_loot() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id, _) = insert_config_loot_if_missing(db.pool(), &test_location(), "health_tonic")
            .await
            .unwrap();
        mark_taken(db.pool(), id).await.unwrap();

        let (id2, is_new2) =
            insert_config_loot_if_missing(db.pool(), &test_location(), "health_tonic")
                .await
                .unwrap();
        assert!(!is_new2);
        assert_eq!(id, id2);

        let found = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn delete_by_room_removes_only_that_room() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        insert_config_loot_if_missing(db.pool(), &test_location(), "health_tonic")
            .await
            .unwrap();
        insert_config_loot_if_missing(db.pool(), &other_location(), "health_tonic")
            .await
            .unwrap();

        delete_by_room(db.pool(), "r1").await.unwrap();

        assert!(
            find_by_location(db.pool(), &test_location())
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            find_by_location(db.pool(), &other_location())
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn respawn_room_only_respawns_that_room() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id1, _) = insert_config_loot_if_missing(db.pool(), &test_location(), "health_tonic")
            .await
            .unwrap();
        let (id2, _) = insert_config_loot_if_missing(db.pool(), &other_location(), "health_tonic")
            .await
            .unwrap();
        mark_taken(db.pool(), id1).await.unwrap();
        mark_taken(db.pool(), id2).await.unwrap();

        respawn_room(db.pool(), &test_location()).await.unwrap();

        assert_eq!(
            find_by_location(db.pool(), &test_location())
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(
            find_by_location(db.pool(), &other_location())
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn respawn_dungeon_respawns_all_rooms_in_dungeon() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id1, _) = insert_config_loot_if_missing(db.pool(), &test_location(), "health_tonic")
            .await
            .unwrap();
        let (id2, _) = insert_config_loot_if_missing(db.pool(), &other_location(), "health_tonic")
            .await
            .unwrap();
        mark_taken(db.pool(), id1).await.unwrap();
        mark_taken(db.pool(), id2).await.unwrap();

        respawn_dungeon(db.pool(), "w1", "d1").await.unwrap();

        assert_eq!(
            find_by_location(db.pool(), &test_location())
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            find_by_location(db.pool(), &other_location())
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn respawn_all_config_seeded_respawns_everything() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id1, _) = insert_config_loot_if_missing(db.pool(), &test_location(), "health_tonic")
            .await
            .unwrap();
        mark_taken(db.pool(), id1).await.unwrap();

        respawn_all_config_seeded(db.pool()).await.unwrap();

        assert_eq!(
            find_by_location(db.pool(), &test_location())
                .await
                .unwrap()
                .len(),
            1
        );
    }
}
