use sqlx::SqlitePool;

use crate::game::component::Interaction;
use crate::persistence::error::PersistenceError;

pub async fn insert(
    pool: &SqlitePool,
    entity_id: i64,
    interaction: &Interaction,
) -> Result<(), PersistenceError> {
    let json = serde_json::to_string(interaction).map_err(PersistenceError::Json)?;
    sqlx::query("INSERT INTO interactions (entity_id, interaction_json) VALUES (?, ?)")
        .bind(entity_id)
        .bind(json)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_by_entity(
    pool: &SqlitePool,
    entity_id: i64,
) -> Result<Vec<Interaction>, PersistenceError> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT interaction_json FROM interactions WHERE entity_id = ?")
            .bind(entity_id)
            .fetch_all(pool)
            .await?;

    rows.into_iter()
        .map(|(json,)| serde_json::from_str(&json).map_err(PersistenceError::Json))
        .collect()
}

pub async fn delete_by_entity(pool: &SqlitePool, entity_id: i64) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM interactions WHERE entity_id = ?")
        .bind(entity_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::interaction::{Direction, Movement};
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
        let entity = Entity::new(0, EntityType::Player, loc);
        entity_repo::insert(db.pool(), &entity).await.unwrap()
    }

    #[tokio::test]
    async fn insert_and_find_by_entity() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        let interaction = Interaction::Movement(Movement::TryDirection(Direction::North));
        insert(db.pool(), entity_id, &interaction).await.unwrap();

        let found = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0], interaction);
    }

    #[tokio::test]
    async fn delete_by_entity_removes_interactions() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        insert(
            db.pool(),
            entity_id,
            &Interaction::Movement(Movement::TryDirection(Direction::East)),
        )
        .await
        .unwrap();
        insert(
            db.pool(),
            entity_id,
            &Interaction::Movement(Movement::TryDirection(Direction::West)),
        )
        .await
        .unwrap();

        delete_by_entity(db.pool(), entity_id).await.unwrap();

        let found = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn cascade_delete_on_entity_delete() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        insert(
            db.pool(),
            entity_id,
            &Interaction::Movement(Movement::TryDirection(Direction::South)),
        )
        .await
        .unwrap();

        entity_repo::delete(db.pool(), entity_id).await.unwrap();

        let found = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert!(found.is_empty());
    }
}
