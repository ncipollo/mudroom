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
    use std::borrow::Cow;

    use sqlx::SqlitePool;
    use sqlx::migrate::Migrator;

    use super::*;

    async fn bare_pool() -> SqlitePool {
        SqlitePool::connect("sqlite::memory:").await.unwrap()
    }

    async fn run_up_to(pool: &SqlitePool, version: i64) {
        let all = sqlx::migrate!("./migrations");
        let migrator = Migrator {
            migrations: Cow::Owned(
                all.iter()
                    .filter(|m| m.version <= version)
                    .cloned()
                    .collect(),
            ),
            ignore_missing: false,
            locking: true,
            no_tx: false,
        };
        migrator.run(pool).await.unwrap();
    }

    async fn table_exists(pool: &SqlitePool, name: &str) -> bool {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?")
                .bind(name)
                .fetch_one(pool)
                .await
                .unwrap();
        row.0 == 1
    }

    async fn index_exists(pool: &SqlitePool, name: &str) -> bool {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name = ?")
                .bind(name)
                .fetch_one(pool)
                .await
                .unwrap();
        row.0 == 1
    }

    #[tokio::test]
    async fn connect_in_memory_creates_all_tables() {
        let db = Database::connect_in_memory().await.unwrap();
        let pool = db.pool();
        for table in &[
            "worlds",
            "dungeons",
            "rooms",
            "entities",
            "attributes",
            "interactions",
            "players",
            "server_state",
        ] {
            assert!(table_exists(pool, table).await, "missing table: {table}");
        }
        assert!(index_exists(pool, "idx_players_client_id").await);
    }

    #[tokio::test]
    async fn migration_1_creates_core_tables() {
        let pool = bare_pool().await;
        run_up_to(&pool, 1).await;
        for table in &["worlds", "dungeons", "rooms", "entities", "attributes"] {
            assert!(table_exists(&pool, table).await, "missing table: {table}");
        }
        assert!(!table_exists(&pool, "interactions").await);
        assert!(!table_exists(&pool, "players").await);
    }

    #[tokio::test]
    async fn migration_2_adds_interactions_table() {
        let pool = bare_pool().await;
        run_up_to(&pool, 1).await;
        assert!(!table_exists(&pool, "interactions").await);

        run_up_to(&pool, 2).await;
        assert!(table_exists(&pool, "interactions").await);
    }

    #[tokio::test]
    async fn migration_3_adds_players_table_and_index() {
        let pool = bare_pool().await;
        run_up_to(&pool, 2).await;
        assert!(!table_exists(&pool, "players").await);

        run_up_to(&pool, 3).await;
        assert!(table_exists(&pool, "players").await);
        assert!(index_exists(&pool, "idx_players_client_id").await);
    }

    #[tokio::test]
    async fn migration_4_adds_server_state_table() {
        let pool = bare_pool().await;
        run_up_to(&pool, 3).await;
        assert!(!table_exists(&pool, "server_state").await);

        run_up_to(&pool, 4).await;
        assert!(table_exists(&pool, "server_state").await);
    }
}
