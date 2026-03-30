use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::game::component::Attribute;
use crate::game::{Entity, EntityType, Location};
use crate::persistence::error::PersistenceError;

type EntityRow = (
    i64,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
);

pub async fn insert(pool: &SqlitePool, entity: &Entity) -> Result<i64, PersistenceError> {
    let entity_type = entity_type_to_str(&entity.entity_type);
    let attributes_json = serde_json::to_string(&entity.attributes)?;
    let result = sqlx::query(
        "INSERT INTO entities (entity_type, world_id, dungeon_id, room_id, config_id, attributes) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(entity_type)
    .bind(&entity.location.world_id)
    .bind(&entity.location.dungeon_id)
    .bind(&entity.location.room_id)
    .bind(&entity.config_id)
    .bind(attributes_json)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

/// Insert a config entity if no entity with the same config_id + original location exists.
/// Returns `(entity_id, is_new)` — id of the existing or newly inserted entity, and whether it
/// was just created. Current location is preserved on conflict (entity may have moved).
pub async fn insert_config_entity_if_missing(
    pool: &SqlitePool,
    entity_type: &EntityType,
    location: &Location,
    config_id: &str,
) -> Result<(i64, bool), PersistenceError> {
    let entity_type_str = entity_type_to_str(entity_type);

    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM entities
         WHERE config_id = ? AND original_world_id = ? AND original_dungeon_id = ? AND original_room_id = ?",
    )
    .bind(config_id)
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .fetch_optional(pool)
    .await?;

    if let Some((id,)) = existing {
        sqlx::query("UPDATE entities SET entity_type = ? WHERE id = ?")
            .bind(entity_type_str)
            .bind(id)
            .execute(pool)
            .await?;
        return Ok((id, false));
    }

    let result = sqlx::query(
        "INSERT INTO entities
             (entity_type, world_id, dungeon_id, room_id, config_id,
              original_world_id, original_dungeon_id, original_room_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(entity_type_str)
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .bind(config_id)
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .execute(pool)
    .await?;

    Ok((result.last_insert_rowid(), true))
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Entity>, PersistenceError> {
    let row: Option<EntityRow> = sqlx::query_as(
        "SELECT id, entity_type, world_id, dungeon_id, room_id, config_id, attributes FROM entities WHERE id = ?",
    )
        .bind(id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(
        |(id, et, world_id, dungeon_id, room_id, config_id, attrs_json)| {
            let attributes = attrs_json
                .and_then(|json| match serde_json::from_str(&json) {
                    Ok(v) => Some(v),
                    Err(e) => {
                        tracing::warn!("Failed to deserialize attributes for entity {id}: {e}");
                        None
                    }
                })
                .unwrap_or_default();
            let mut entity = Entity::new(
                id,
                entity_type_from_str(&et),
                Location {
                    world_id,
                    dungeon_id,
                    room_id,
                },
            );
            entity.config_id = config_id;
            entity.attributes = attributes;
            entity
        },
    ))
}

pub async fn find_by_location(
    pool: &SqlitePool,
    location: &Location,
) -> Result<Vec<Entity>, PersistenceError> {
    let rows: Vec<EntityRow> = sqlx::query_as(
        "SELECT id, entity_type, world_id, dungeon_id, room_id, config_id, attributes FROM entities WHERE world_id = ? AND dungeon_id = ? AND room_id = ?",
    )
        .bind(&location.world_id)
        .bind(&location.dungeon_id)
        .bind(&location.room_id)
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(
            |(id, et, world_id, dungeon_id, room_id, config_id, attrs_json)| {
                let attributes = attrs_json
                    .and_then(|json| match serde_json::from_str(&json) {
                        Ok(v) => Some(v),
                        Err(e) => {
                            tracing::warn!("Failed to deserialize attributes for entity {id}: {e}");
                            None
                        }
                    })
                    .unwrap_or_default();
                let mut entity = Entity::new(
                    id,
                    entity_type_from_str(&et),
                    Location {
                        world_id,
                        dungeon_id,
                        room_id,
                    },
                );
                entity.config_id = config_id;
                entity.attributes = attributes;
                entity
            },
        )
        .collect())
}

pub async fn find_config_entities_by_dungeon(
    pool: &SqlitePool,
    world_id: &str,
    dungeon_id: &str,
) -> Result<Vec<Entity>, PersistenceError> {
    let rows: Vec<EntityRow> = sqlx::query_as(
        "SELECT id, entity_type, world_id, dungeon_id, room_id, config_id, attributes FROM entities WHERE config_id IS NOT NULL AND world_id = ? AND dungeon_id = ?",
    )
    .bind(world_id)
    .bind(dungeon_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(id, et, world_id, dungeon_id, room_id, config_id, attrs_json)| {
                let attributes = attrs_json
                    .and_then(|json| match serde_json::from_str(&json) {
                        Ok(v) => Some(v),
                        Err(e) => {
                            tracing::warn!("Failed to deserialize attributes for entity {id}: {e}");
                            None
                        }
                    })
                    .unwrap_or_default();
                let mut entity = Entity::new(
                    id,
                    entity_type_from_str(&et),
                    Location {
                        world_id,
                        dungeon_id,
                        room_id,
                    },
                );
                entity.config_id = config_id;
                entity.attributes = attributes;
                entity
            },
        )
        .collect())
}

pub async fn update_attributes(
    pool: &SqlitePool,
    entity_id: i64,
    attributes: &HashMap<String, Attribute>,
) -> Result<(), PersistenceError> {
    let json = serde_json::to_string(attributes)?;
    sqlx::query("UPDATE entities SET attributes = ? WHERE id = ?")
        .bind(json)
        .bind(entity_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_location(
    pool: &SqlitePool,
    entity_id: i64,
    location: &Location,
) -> Result<(), PersistenceError> {
    sqlx::query("UPDATE entities SET world_id = ?, dungeon_id = ?, room_id = ? WHERE id = ?")
        .bind(&location.world_id)
        .bind(&location.dungeon_id)
        .bind(&location.room_id)
        .bind(entity_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM entities WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_by_room(pool: &SqlitePool, room_id: &str) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM entities WHERE room_id = ?")
        .bind(room_id)
        .execute(pool)
        .await?;
    Ok(())
}

fn entity_type_to_str(et: &EntityType) -> &'static str {
    match et {
        EntityType::Player => "player",
        EntityType::Character => "character",
        EntityType::Object => "object",
    }
}

fn entity_type_from_str(s: &str) -> EntityType {
    match s {
        "player" => EntityType::Player,
        "object" => EntityType::Object,
        _ => EntityType::Character,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Description, Room};
    use crate::game::{Dungeon, World};
    use crate::persistence::database::Database;
    use crate::persistence::{dungeon_repo, room_repo, world_repo};

    async fn setup(db: &Database) {
        let world = World::new("w1".to_string());
        world_repo::insert(db.pool(), &world).await.unwrap();
        let dungeon = Dungeon::new("d1".to_string());
        dungeon_repo::insert(db.pool(), &dungeon, "w1")
            .await
            .unwrap();
        let room = Room::new("r1".to_string(), Description::new(None));
        room_repo::insert(db.pool(), &room, "d1").await.unwrap();
    }

    fn test_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    #[tokio::test]
    async fn insert_and_find_by_id() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        let entity = Entity::new(0, EntityType::Player, test_location());
        let id = insert(db.pool(), &entity).await.unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.id, id);
        assert_eq!(found.location.world_id, "w1");
    }

    #[tokio::test]
    async fn find_by_id_returns_none_for_missing() {
        let db = Database::connect_in_memory().await.unwrap();
        let found = find_by_id(db.pool(), 999).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn find_by_location_returns_entities() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        insert(
            db.pool(),
            &Entity::new(0, EntityType::Player, test_location()),
        )
        .await
        .unwrap();
        insert(
            db.pool(),
            &Entity::new(0, EntityType::Character, test_location()),
        )
        .await
        .unwrap();

        let entities = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert_eq!(entities.len(), 2);
    }

    #[tokio::test]
    async fn update_location_changes_location() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        // Add a second room for the new location
        let room2 = Room::new("r2".to_string(), Description::new(None));
        room_repo::insert(db.pool(), &room2, "d1").await.unwrap();

        let entity = Entity::new(0, EntityType::Player, test_location());
        let id = insert(db.pool(), &entity).await.unwrap();

        let new_loc = Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r2".to_string(),
        };
        update_location(db.pool(), id, &new_loc).await.unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.location.room_id, "r2");
    }

    #[tokio::test]
    async fn delete_removes_entity() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        let entity = Entity::new(0, EntityType::Player, test_location());
        let id = insert(db.pool(), &entity).await.unwrap();
        delete(db.pool(), id).await.unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn delete_by_room_removes_all_entities_in_room() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        insert(
            db.pool(),
            &Entity::new(0, EntityType::Player, test_location()),
        )
        .await
        .unwrap();
        insert(
            db.pool(),
            &Entity::new(0, EntityType::Character, test_location()),
        )
        .await
        .unwrap();

        delete_by_room(db.pool(), "r1").await.unwrap();

        let entities = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert!(entities.is_empty());
    }

    #[tokio::test]
    async fn find_config_entities_by_dungeon_returns_matching() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id, _) = insert_config_entity_if_missing(
            db.pool(),
            &EntityType::Character,
            &test_location(),
            "entities/innkeeper",
        )
        .await
        .unwrap();

        // Player entity (no config_id) should not be returned
        insert(
            db.pool(),
            &Entity::new(0, EntityType::Player, test_location()),
        )
        .await
        .unwrap();

        let found = find_config_entities_by_dungeon(db.pool(), "w1", "d1")
            .await
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, id);

        // Different dungeon returns nothing
        let other = find_config_entities_by_dungeon(db.pool(), "w1", "d2")
            .await
            .unwrap();
        assert!(other.is_empty());
    }

    #[tokio::test]
    async fn insert_config_entity_if_missing_inserts_new() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id, is_new) = insert_config_entity_if_missing(
            db.pool(),
            &EntityType::Character,
            &test_location(),
            "entities/innkeeper",
        )
        .await
        .unwrap();
        assert!(id > 0);
        assert!(is_new);

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.config_id.as_deref(), Some("entities/innkeeper"));
    }

    #[tokio::test]
    async fn insert_config_entity_if_missing_returns_existing_id() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let (id1, is_new1) = insert_config_entity_if_missing(
            db.pool(),
            &EntityType::Character,
            &test_location(),
            "entities/innkeeper",
        )
        .await
        .unwrap();
        let (id2, is_new2) = insert_config_entity_if_missing(
            db.pool(),
            &EntityType::Character,
            &test_location(),
            "entities/innkeeper",
        )
        .await
        .unwrap();
        assert_eq!(id1, id2);
        assert!(is_new1);
        assert!(!is_new2);

        let entities = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert_eq!(entities.len(), 1);
    }

    #[tokio::test]
    async fn insert_and_find_preserves_attributes() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let mut entity = Entity::new(0, EntityType::Character, test_location());
        entity.attributes.insert(
            "hp".to_string(),
            Attribute::new("hp".to_string(), 0, 100, 80),
        );
        entity.attributes.insert(
            "mp".to_string(),
            Attribute::new("mp".to_string(), 0, 50, 50),
        );
        let id = insert(db.pool(), &entity).await.unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.attributes.len(), 2);
        assert_eq!(found.attributes["hp"].current_value, 80);
        assert_eq!(found.attributes["mp"].max_value, 50);
    }

    #[tokio::test]
    async fn find_by_id_with_corrupt_attributes_returns_empty() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let entity = Entity::new(0, EntityType::Character, test_location());
        let id = insert(db.pool(), &entity).await.unwrap();

        // Corrupt the attributes column
        sqlx::query("UPDATE entities SET attributes = ? WHERE id = ?")
            .bind("not valid json{{{")
            .bind(id)
            .execute(db.pool())
            .await
            .unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert!(found.attributes.is_empty());
    }

    #[tokio::test]
    async fn update_attributes_persists_changes() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let entity = Entity::new(0, EntityType::Character, test_location());
        let id = insert(db.pool(), &entity).await.unwrap();

        let mut attrs = HashMap::new();
        attrs.insert(
            "str".to_string(),
            Attribute::new("str".to_string(), 1, 20, 15),
        );
        update_attributes(db.pool(), id, &attrs).await.unwrap();

        let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(found.attributes.len(), 1);
        assert_eq!(found.attributes["str"].current_value, 15);
    }
}
