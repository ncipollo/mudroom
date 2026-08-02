use sqlx::SqlitePool;

use crate::game::component::description::Description;
use crate::game::component::resource_definition::{ResourceDefinition, ResourceLifespan};
use crate::persistence::error::PersistenceError;

pub async fn upsert_definition(
    pool: &SqlitePool,
    def: &ResourceDefinition,
) -> Result<(), PersistenceError> {
    let lifespan = serde_json::to_string(&def.lifespan).unwrap_or_default();
    sqlx::query(
        "INSERT OR REPLACE INTO resource_definitions \
         (id, name, description, min_value, max_value, lifespan) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&def.id)
    .bind(&def.name)
    .bind(def.description.text.clone().unwrap_or_default())
    .bind(def.min_value)
    .bind(def.max_value)
    .bind(&lifespan)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_all_definitions(
    pool: &SqlitePool,
) -> Result<Vec<ResourceDefinition>, PersistenceError> {
    let rows: Vec<(String, String, String, i64, i64, String)> = sqlx::query_as(
        "SELECT id, name, description, min_value, max_value, lifespan FROM resource_definitions",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(id, name, description, min_value, max_value, lifespan_str)| {
                let lifespan: ResourceLifespan = serde_json::from_str(&lifespan_str)
                    .unwrap_or_else(|e| {
                        tracing::warn!("Failed to deserialize resource lifespan for {id}: {e}");
                        ResourceLifespan::Perpetual
                    });
                ResourceDefinition {
                    id,
                    name,
                    description: Description::new(Some(description)),
                    min_value,
                    max_value,
                    lifespan,
                }
            },
        )
        .collect())
}

pub async fn upsert_entity_resource(
    pool: &SqlitePool,
    entity_id: i64,
    resource_id: &str,
    current_value: i64,
) -> Result<(), PersistenceError> {
    sqlx::query(
        "INSERT OR REPLACE INTO entity_resources (entity_id, resource_id, current_value) \
         VALUES (?, ?, ?)",
    )
    .bind(entity_id)
    .bind(resource_id)
    .bind(current_value)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_by_entity(
    pool: &SqlitePool,
    entity_id: i64,
) -> Result<Vec<(String, i64)>, PersistenceError> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT resource_id, current_value FROM entity_resources WHERE entity_id = ?",
    )
    .bind(entity_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Dungeon, Entity, EntityType, Location, Room, World};
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

    fn mp_definition() -> ResourceDefinition {
        ResourceDefinition {
            id: "mp".to_string(),
            name: "Mana Points".to_string(),
            description: Description::new(Some("Magical energy.".to_string())),
            min_value: 0,
            max_value: 999,
            lifespan: ResourceLifespan::Perpetual,
        }
    }

    #[tokio::test]
    async fn upsert_and_find_all_definitions() {
        let db = Database::connect_in_memory().await.unwrap();
        upsert_definition(db.pool(), &mp_definition())
            .await
            .unwrap();
        let defs = find_all_definitions(db.pool()).await.unwrap();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].id, "mp");
        assert_eq!(defs[0].lifespan, ResourceLifespan::Perpetual);
    }

    #[tokio::test]
    async fn upsert_definition_is_idempotent() {
        let db = Database::connect_in_memory().await.unwrap();
        upsert_definition(db.pool(), &mp_definition())
            .await
            .unwrap();
        upsert_definition(db.pool(), &mp_definition())
            .await
            .unwrap();
        let defs = find_all_definitions(db.pool()).await.unwrap();
        assert_eq!(defs.len(), 1);
    }

    #[tokio::test]
    async fn upsert_and_find_entity_resource() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        upsert_definition(db.pool(), &mp_definition())
            .await
            .unwrap();
        upsert_entity_resource(db.pool(), entity_id, "mp", 50)
            .await
            .unwrap();

        let resources = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].0, "mp");
        assert_eq!(resources[0].1, 50);
    }

    #[tokio::test]
    async fn cascade_delete_on_entity_delete() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        upsert_definition(db.pool(), &mp_definition())
            .await
            .unwrap();
        upsert_entity_resource(db.pool(), entity_id, "mp", 50)
            .await
            .unwrap();

        entity_repo::delete(db.pool(), entity_id).await.unwrap();

        let resources = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert!(resources.is_empty());
    }
}
