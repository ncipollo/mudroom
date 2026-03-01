use sqlx::SqlitePool;

use crate::game::{Description, Navigation, Room};
use crate::persistence::error::PersistenceError;

type RoomRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

pub async fn insert(
    pool: &SqlitePool,
    room: &Room,
    dungeon_id: &str,
) -> Result<(), PersistenceError> {
    let description_json = serde_json::to_string(&room.description)?;
    let north_json = room.north.as_ref().map(serde_json::to_string).transpose()?;
    let south_json = room.south.as_ref().map(serde_json::to_string).transpose()?;
    let east_json = room.east.as_ref().map(serde_json::to_string).transpose()?;
    let west_json = room.west.as_ref().map(serde_json::to_string).transpose()?;

    sqlx::query(
        "INSERT OR REPLACE INTO rooms (id, dungeon_id, description_json, north_json, south_json, east_json, west_json) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&room.id)
    .bind(dungeon_id)
    .bind(&description_json)
    .bind(&north_json)
    .bind(&south_json)
    .bind(&east_json)
    .bind(&west_json)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Room>, PersistenceError> {
    let row: Option<RoomRow> = sqlx::query_as(
            "SELECT id, description_json, north_json, south_json, east_json, west_json FROM rooms WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

    row.map(|(id, desc_json, north, south, east, west)| {
        parse_room(id, desc_json, north, south, east, west)
    })
    .transpose()
}

pub async fn find_by_dungeon(
    pool: &SqlitePool,
    dungeon_id: &str,
) -> Result<Vec<Room>, PersistenceError> {
    let rows: Vec<RoomRow> = sqlx::query_as(
            "SELECT id, description_json, north_json, south_json, east_json, west_json FROM rooms WHERE dungeon_id = ?",
        )
        .bind(dungeon_id)
        .fetch_all(pool)
        .await?;

    rows.into_iter()
        .map(|(id, desc_json, north, south, east, west)| {
            parse_room(id, desc_json, north, south, east, west)
        })
        .collect()
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM rooms WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

fn parse_room(
    id: String,
    desc_json: String,
    north: Option<String>,
    south: Option<String>,
    east: Option<String>,
    west: Option<String>,
) -> Result<Room, PersistenceError> {
    let description: Description = serde_json::from_str(&desc_json)?;
    let north: Option<Navigation> = north.as_deref().map(serde_json::from_str).transpose()?;
    let south: Option<Navigation> = south.as_deref().map(serde_json::from_str).transpose()?;
    let east: Option<Navigation> = east.as_deref().map(serde_json::from_str).transpose()?;
    let west: Option<Navigation> = west.as_deref().map(serde_json::from_str).transpose()?;
    Ok(Room {
        id,
        description,
        north,
        south,
        east,
        west,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Dungeon, World};
    use crate::persistence::database::Database;
    use crate::persistence::{dungeon_repo, world_repo};

    async fn setup(db: &Database) {
        let world = World::new("w1".to_string());
        world_repo::insert(db.pool(), &world).await.unwrap();
        let dungeon = Dungeon::new("d1".to_string());
        dungeon_repo::insert(db.pool(), &dungeon, "w1")
            .await
            .unwrap();
    }

    fn make_room(id: &str) -> Room {
        Room::new(
            id.to_string(),
            Description::new(Some("A room.".to_string())),
        )
    }

    #[tokio::test]
    async fn insert_and_find_by_id() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        let room = make_room("r1");
        insert(db.pool(), &room, "d1").await.unwrap();

        let found = find_by_id(db.pool(), "r1").await.unwrap().unwrap();
        assert_eq!(found.id, "r1");
        assert_eq!(found.description.basic.as_deref(), Some("A room."));
    }

    #[tokio::test]
    async fn find_by_id_returns_none_for_missing() {
        let db = Database::connect_in_memory().await.unwrap();
        let found = find_by_id(db.pool(), "nonexistent").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn find_by_dungeon_returns_rooms() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        insert(db.pool(), &make_room("r1"), "d1").await.unwrap();
        insert(db.pool(), &make_room("r2"), "d1").await.unwrap();

        let rooms = find_by_dungeon(db.pool(), "d1").await.unwrap();
        assert_eq!(rooms.len(), 2);
    }

    #[tokio::test]
    async fn delete_removes_room() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        insert(db.pool(), &make_room("r1"), "d1").await.unwrap();
        delete(db.pool(), "r1").await.unwrap();

        let found = find_by_id(db.pool(), "r1").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn navigation_survives_round_trip() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        let mut room = make_room("r1");
        room.north = Some(Navigation {
            world_id: Some("w1".to_string()),
            dungeon_id: Some("d1".to_string()),
            room_id: Some("r2".to_string()),
        });
        insert(db.pool(), &room, "d1").await.unwrap();

        let found = find_by_id(db.pool(), "r1").await.unwrap().unwrap();
        let north = found.north.unwrap();
        assert_eq!(north.world_id.as_deref(), Some("w1"));
        assert_eq!(north.room_id.as_deref(), Some("r2"));
    }
}
