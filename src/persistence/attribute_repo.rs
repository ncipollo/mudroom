use sqlx::SqlitePool;

use crate::game::component::Attribute;
use crate::persistence::error::PersistenceError;

pub async fn insert(
    pool: &SqlitePool,
    entity_id: i64,
    attribute: &Attribute,
) -> Result<(), PersistenceError> {
    sqlx::query(
        "INSERT INTO attributes (entity_id, definition_id, min_value, max_value, current_value) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(entity_id)
    .bind(&attribute.definition_id)
    .bind(attribute.min_value)
    .bind(attribute.max_value)
    .bind(attribute.current_value)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_by_entity(
    pool: &SqlitePool,
    entity_id: i64,
) -> Result<Vec<Attribute>, PersistenceError> {
    let rows: Vec<(String, i64, i64, i64)> = sqlx::query_as(
        "SELECT definition_id, min_value, max_value, current_value FROM attributes WHERE entity_id = ?",
    )
    .bind(entity_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(def_id, min, max, current)| Attribute::new(def_id, min, max, current))
        .collect())
}

pub async fn delete_by_entity(pool: &SqlitePool, entity_id: i64) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM attributes WHERE entity_id = ?")
        .bind(entity_id)
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
        let loc = Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        };
        let entity = Entity::new(42, EntityType::Player, loc);
        entity_repo::insert(db.pool(), &entity).await.unwrap();
        42
    }

    #[tokio::test]
    async fn insert_and_find_by_entity() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        let attr = Attribute::new("hp".to_string(), 0, 100, 80);
        insert(db.pool(), entity_id, &attr).await.unwrap();

        let attrs = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].definition_id, "hp");
        assert_eq!(attrs[0].current_value, 80);
    }

    #[tokio::test]
    async fn delete_by_entity_removes_attributes() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        insert(
            db.pool(),
            entity_id,
            &Attribute::new("hp".to_string(), 0, 100, 50),
        )
        .await
        .unwrap();
        insert(
            db.pool(),
            entity_id,
            &Attribute::new("mp".to_string(), 0, 50, 30),
        )
        .await
        .unwrap();

        delete_by_entity(db.pool(), entity_id).await.unwrap();

        let attrs = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert!(attrs.is_empty());
    }

    #[tokio::test]
    async fn cascade_delete_on_entity_delete() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        insert(
            db.pool(),
            entity_id,
            &Attribute::new("hp".to_string(), 0, 100, 50),
        )
        .await
        .unwrap();

        entity_repo::delete(db.pool(), entity_id).await.unwrap();

        let attrs = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert!(attrs.is_empty());
    }
}
