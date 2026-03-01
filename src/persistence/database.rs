use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::str::FromStr;

use crate::persistence::error::PersistenceError;
use crate::state::config::database_url;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn connect() -> Result<Self, PersistenceError> {
        let url = database_url().map_err(|_| PersistenceError::NoHomeDir)?;
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

    #[tokio::test]
    async fn connect_in_memory_runs_migrations() {
        let db = Database::connect_in_memory().await.unwrap();
        // Verify that the tables exist by querying sqlite_master
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('worlds','dungeons','rooms','entities','attributes')")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(count.0, 5);
    }
}
