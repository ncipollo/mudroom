use sqlx::SqlitePool;

use crate::persistence::error::PersistenceError;

pub async fn get(pool: &SqlitePool, key: &str) -> Result<Option<String>, PersistenceError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM server_state WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(v,)| v))
}

pub async fn set(pool: &SqlitePool, key: &str, value: &str) -> Result<(), PersistenceError> {
    sqlx::query("INSERT OR REPLACE INTO server_state (key, value) VALUES (?, ?)")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::database::Database;

    #[tokio::test]
    async fn get_returns_none_for_missing_key() {
        let db = Database::connect_in_memory().await.unwrap();
        let result = get(db.pool(), "missing").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn set_and_get_round_trip() {
        let db = Database::connect_in_memory().await.unwrap();
        set(db.pool(), "foo", "bar").await.unwrap();
        let result = get(db.pool(), "foo").await.unwrap();
        assert_eq!(result.as_deref(), Some("bar"));
    }

    #[tokio::test]
    async fn set_overwrites_existing_value() {
        let db = Database::connect_in_memory().await.unwrap();
        set(db.pool(), "key", "first").await.unwrap();
        set(db.pool(), "key", "second").await.unwrap();
        let result = get(db.pool(), "key").await.unwrap();
        assert_eq!(result.as_deref(), Some("second"));
    }
}
