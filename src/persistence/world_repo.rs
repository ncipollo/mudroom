use sqlx::SqlitePool;

use crate::game::World;
use crate::persistence::error::PersistenceError;

pub async fn insert(pool: &SqlitePool, world: &World) -> Result<(), PersistenceError> {
    sqlx::query("INSERT OR REPLACE INTO worlds (id) VALUES (?)")
        .bind(&world.id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<World>, PersistenceError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT id FROM worlds WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(id,)| World::new(id)))
}

pub async fn find_all(pool: &SqlitePool) -> Result<Vec<World>, PersistenceError> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM worlds")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|(id,)| World::new(id)).collect())
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM worlds WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::database::Database;

    #[tokio::test]
    async fn insert_and_find_by_id() {
        let db = Database::connect_in_memory().await.unwrap();
        let world = World::new("world-1".to_string());
        insert(db.pool(), &world).await.unwrap();

        let found = find_by_id(db.pool(), "world-1").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "world-1");
    }

    #[tokio::test]
    async fn find_by_id_returns_none_for_missing() {
        let db = Database::connect_in_memory().await.unwrap();
        let found = find_by_id(db.pool(), "nonexistent").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn find_all_returns_all_worlds() {
        let db = Database::connect_in_memory().await.unwrap();
        insert(db.pool(), &World::new("w1".to_string()))
            .await
            .unwrap();
        insert(db.pool(), &World::new("w2".to_string()))
            .await
            .unwrap();

        let all = find_all(db.pool()).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn delete_removes_world() {
        let db = Database::connect_in_memory().await.unwrap();
        insert(db.pool(), &World::new("w1".to_string()))
            .await
            .unwrap();
        delete(db.pool(), "w1").await.unwrap();

        let found = find_by_id(db.pool(), "w1").await.unwrap();
        assert!(found.is_none());
    }
}
