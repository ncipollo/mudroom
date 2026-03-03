use sqlx::SqlitePool;

use crate::game::Dungeon;
use crate::persistence::error::PersistenceError;

pub async fn insert(
    pool: &SqlitePool,
    dungeon: &Dungeon,
    world_id: &str,
) -> Result<(), PersistenceError> {
    sqlx::query("INSERT OR REPLACE INTO dungeons (id, world_id) VALUES (?, ?)")
        .bind(&dungeon.id)
        .bind(world_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Dungeon>, PersistenceError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT id FROM dungeons WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(id,)| Dungeon::new(id)))
}

pub async fn find_by_world(
    pool: &SqlitePool,
    world_id: &str,
) -> Result<Vec<Dungeon>, PersistenceError> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM dungeons WHERE world_id = ?")
        .bind(world_id)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|(id,)| Dungeon::new(id)).collect())
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM dungeons WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::World;
    use crate::persistence::database::Database;
    use crate::persistence::world_repo;

    async fn setup_world(db: &Database) -> World {
        let world = World::new("world-1".to_string());
        world_repo::insert(db.pool(), &world).await.unwrap();
        world
    }

    #[tokio::test]
    async fn insert_and_find_by_id() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let dungeon = Dungeon::new("d1".to_string());
        insert(db.pool(), &dungeon, "world-1").await.unwrap();

        let found = find_by_id(db.pool(), "d1").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "d1");
    }

    #[tokio::test]
    async fn find_by_id_returns_none_for_missing() {
        let db = Database::connect_in_memory().await.unwrap();
        let found = find_by_id(db.pool(), "nonexistent").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn find_by_world_returns_dungeons() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        insert(db.pool(), &Dungeon::new("d1".to_string()), "world-1")
            .await
            .unwrap();
        insert(db.pool(), &Dungeon::new("d2".to_string()), "world-1")
            .await
            .unwrap();

        let dungeons = find_by_world(db.pool(), "world-1").await.unwrap();
        assert_eq!(dungeons.len(), 2);
    }

    #[tokio::test]
    async fn delete_removes_dungeon() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        insert(db.pool(), &Dungeon::new("d1".to_string()), "world-1")
            .await
            .unwrap();
        delete(db.pool(), "d1").await.unwrap();

        let found = find_by_id(db.pool(), "d1").await.unwrap();
        assert!(found.is_none());
    }
}
