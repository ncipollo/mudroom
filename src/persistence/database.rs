use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::str::FromStr;

use crate::persistence::error::PersistenceError;
use crate::state::config::{database_url, server_session_dir};

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn connect(server_name: &str) -> Result<Self, PersistenceError> {
        let dir = server_session_dir(server_name).map_err(|_| PersistenceError::NoHomeDir)?;
        tokio::fs::create_dir_all(&dir).await?;
        let url = database_url(server_name).map_err(|_| PersistenceError::NoHomeDir)?;
        Self::connect_with_url(&url).await
    }

    async fn connect_with_url(url: &str) -> Result<Self, PersistenceError> {
        let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[cfg(test)]
impl Database {
    pub async fn connect_in_memory() -> Result<Self, PersistenceError> {
        Self::connect_with_url("sqlite::memory:").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn table_exists(db: &Database, name: &str) -> bool {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?")
                .bind(name)
                .fetch_one(db.pool())
                .await
                .unwrap();
        row.0 == 1
    }

    async fn index_exists(db: &Database, name: &str) -> bool {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name = ?")
                .bind(name)
                .fetch_one(db.pool())
                .await
                .unwrap();
        row.0 == 1
    }

    #[tokio::test]
    async fn migration_1_creates_core_tables() {
        let db = Database::connect_in_memory().await.unwrap();
        for table in &["worlds", "dungeons", "rooms", "entities", "attributes"] {
            assert!(table_exists(&db, table).await, "missing table: {table}");
        }
    }

    #[tokio::test]
    async fn migration_2_creates_interactions_table() {
        let db = Database::connect_in_memory().await.unwrap();
        assert!(table_exists(&db, "interactions").await);
    }

    #[tokio::test]
    async fn migration_3_creates_players_table_and_index() {
        let db = Database::connect_in_memory().await.unwrap();
        assert!(table_exists(&db, "players").await);
        assert!(index_exists(&db, "idx_players_client_id").await);
    }
}
