use sqlx::SqlitePool;

use crate::game::Player;
use crate::persistence::error::PersistenceError;

pub async fn insert(
    pool: &SqlitePool,
    client_id: &str,
    name: &str,
    entity_id: i64,
) -> Result<i64, PersistenceError> {
    let result = sqlx::query("INSERT INTO players (client_id, name, entity_id) VALUES (?, ?, ?)")
        .bind(client_id)
        .bind(name)
        .bind(entity_id)
        .execute(pool)
        .await?;
    Ok(result.last_insert_rowid())
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Player>, PersistenceError> {
    let row: Option<(i64, String, String, i64)> =
        sqlx::query_as("SELECT id, client_id, name, entity_id FROM players WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(id, client_id, name, entity_id)| Player {
        id,
        client_id,
        name,
        entity_id,
    }))
}

pub async fn find_by_client_id(
    pool: &SqlitePool,
    client_id: &str,
) -> Result<Vec<Player>, PersistenceError> {
    let rows: Vec<(i64, String, String, i64)> =
        sqlx::query_as("SELECT id, client_id, name, entity_id FROM players WHERE client_id = ?")
            .bind(client_id)
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(id, client_id, name, entity_id)| Player {
            id,
            client_id,
            name,
            entity_id,
        })
        .collect())
}

pub async fn find_by_entity_id(
    pool: &SqlitePool,
    entity_id: i64,
) -> Result<Option<Player>, PersistenceError> {
    let row: Option<(i64, String, String, i64)> =
        sqlx::query_as("SELECT id, client_id, name, entity_id FROM players WHERE entity_id = ?")
            .bind(entity_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(id, client_id, name, entity_id)| Player {
        id,
        client_id,
        name,
        entity_id,
    }))
}

pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM players WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Description, Dungeon, Entity, EntityType, Location, Room, World};
    use crate::persistence::database::Database;
    use crate::persistence::{dungeon_repo, entity_repo, room_repo, world_repo};

    async fn setup(db: &Database) -> i64 {
        let world = World::new("w1".to_string());
        world_repo::insert(db.pool(), &world).await.unwrap();
        let dungeon = Dungeon::new("d1".to_string());
        dungeon_repo::insert(db.pool(), &dungeon, "w1")
            .await
            .unwrap();
        let room = Room::new("r1".to_string(), Description::new(None));
        room_repo::insert(db.pool(), &room, "d1").await.unwrap();

        let location = Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        };
        let entity = Entity::new(0, EntityType::Player, location);
        entity_repo::insert(db.pool(), &entity).await.unwrap()
    }

    #[tokio::test]
    async fn insert_and_find_by_id() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        let player_id = insert(db.pool(), "client1", "Alice", entity_id)
            .await
            .unwrap();

        let found = find_by_id(db.pool(), player_id).await.unwrap().unwrap();
        assert_eq!(found.id, player_id);
        assert_eq!(found.client_id, "client1");
        assert_eq!(found.name, "Alice");
        assert_eq!(found.entity_id, entity_id);
    }

    #[tokio::test]
    async fn find_by_id_returns_none_for_missing() {
        let db = Database::connect_in_memory().await.unwrap();
        let found = find_by_id(db.pool(), 999).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn find_by_client_id_returns_players() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        insert(db.pool(), "client1", "Alice", entity_id)
            .await
            .unwrap();

        let players = find_by_client_id(db.pool(), "client1").await.unwrap();
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].name, "Alice");
    }

    #[tokio::test]
    async fn find_by_client_id_returns_empty_for_unknown() {
        let db = Database::connect_in_memory().await.unwrap();
        let players = find_by_client_id(db.pool(), "nobody").await.unwrap();
        assert!(players.is_empty());
    }

    #[tokio::test]
    async fn find_by_entity_id_returns_player() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        insert(db.pool(), "client1", "Alice", entity_id)
            .await
            .unwrap();

        let found = find_by_entity_id(db.pool(), entity_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.name, "Alice");
    }

    #[tokio::test]
    async fn find_by_entity_id_returns_none_for_missing() {
        let db = Database::connect_in_memory().await.unwrap();
        let found = find_by_entity_id(db.pool(), 999).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn delete_removes_player() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        let player_id = insert(db.pool(), "client1", "Alice", entity_id)
            .await
            .unwrap();
        delete(db.pool(), player_id).await.unwrap();

        let found = find_by_id(db.pool(), player_id).await.unwrap();
        assert!(found.is_none());
    }
}
