use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::game::component::Location;
use crate::game::{FeatureState, RoomFeature, RoomFeatureState};
use crate::persistence::error::PersistenceError;

type RoomFeatureRow = (i64, String, String, String, String, String);
type FeatureDefinitionRow = (String, String, String, String);

pub async fn upsert_definition(
    pool: &SqlitePool,
    feature: &RoomFeature,
) -> Result<(), PersistenceError> {
    let states_json = serde_json::to_string(&feature.states)?;
    sqlx::query(
        "INSERT INTO feature_definitions (id, name, default_state, states_json) \
         VALUES (?, ?, ?, ?) \
         ON CONFLICT(id) DO UPDATE SET \
             name = excluded.name, \
             default_state = excluded.default_state, \
             states_json = excluded.states_json",
    )
    .bind(&feature.id)
    .bind(&feature.name)
    .bind(&feature.default_state)
    .bind(&states_json)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_definition_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<RoomFeature>, PersistenceError> {
    let row: Option<FeatureDefinitionRow> = sqlx::query_as(
        "SELECT id, name, default_state, states_json FROM feature_definitions WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(parse_feature_definition).transpose()
}

fn parse_feature_definition(row: FeatureDefinitionRow) -> Result<RoomFeature, PersistenceError> {
    let (id, name, default_state, states_json) = row;
    let states: HashMap<String, FeatureState> = serde_json::from_str(&states_json)?;
    Ok(RoomFeature {
        id,
        name,
        default_state,
        states,
    })
}

/// Insert a placement if none exists yet for this (location, feature) pair. Returns
/// `(id, is_new)`. An existing row is left untouched — its `current_state` may have been
/// changed at runtime and must survive a resync.
pub async fn insert_placement_if_missing(
    pool: &SqlitePool,
    location: &Location,
    feature_definition_id: &str,
    default_state: &str,
) -> Result<(i64, bool), PersistenceError> {
    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM room_features \
         WHERE world_id = ? AND dungeon_id = ? AND room_id = ? AND feature_definition_id = ?",
    )
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .bind(feature_definition_id)
    .fetch_optional(pool)
    .await?;

    if let Some((id,)) = existing {
        return Ok((id, false));
    }

    let result = sqlx::query(
        "INSERT INTO room_features \
             (feature_definition_id, world_id, dungeon_id, room_id, current_state) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(feature_definition_id)
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .bind(default_state)
    .execute(pool)
    .await?;

    Ok((result.last_insert_rowid(), true))
}

pub async fn find_by_location(
    pool: &SqlitePool,
    location: &Location,
) -> Result<Vec<RoomFeatureState>, PersistenceError> {
    let rows: Vec<RoomFeatureRow> = sqlx::query_as(
        "SELECT id, feature_definition_id, world_id, dungeon_id, room_id, current_state \
         FROM room_features WHERE world_id = ? AND dungeon_id = ? AND room_id = ?",
    )
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(parse_room_feature_state).collect())
}

fn parse_room_feature_state(row: RoomFeatureRow) -> RoomFeatureState {
    let (id, feature_definition_id, world_id, dungeon_id, room_id, current_state) = row;
    RoomFeatureState::new(
        id,
        feature_definition_id,
        Location {
            world_id,
            dungeon_id,
            room_id,
        },
        current_state,
    )
}

pub async fn update_state(
    pool: &SqlitePool,
    id: i64,
    new_state: &str,
) -> Result<(), PersistenceError> {
    sqlx::query("UPDATE room_features SET current_state = ? WHERE id = ?")
        .bind(new_state)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_by_room(pool: &SqlitePool, room_id: &str) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM room_features WHERE room_id = ?")
        .bind(room_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Description, Dungeon, Room, World};
    use crate::persistence::database::Database;
    use crate::persistence::{dungeon_repo, room_repo, world_repo};

    fn test_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    fn other_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r2".to_string(),
        }
    }

    fn chest_feature() -> RoomFeature {
        let mut states = HashMap::new();
        states.insert(
            "closed".to_string(),
            FeatureState {
                description: crate::game::component::description::Description::new(Some(
                    "A closed chest.".to_string(),
                )),
                items: vec![],
                interact_script: None,
                interact_next_state: Some("open".to_string()),
            },
        );
        states.insert(
            "open".to_string(),
            FeatureState {
                description: crate::game::component::description::Description::new(Some(
                    "An open chest.".to_string(),
                )),
                items: vec!["medicine".to_string()],
                interact_script: None,
                interact_next_state: None,
            },
        );
        RoomFeature {
            id: "chest".to_string(),
            name: "Oak Chest".to_string(),
            default_state: "closed".to_string(),
            states,
        }
    }

    async fn setup(db: &Database) {
        let world = World::new("w1".to_string());
        world_repo::insert(db.pool(), &world).await.unwrap();
        let dungeon = Dungeon::new("d1".to_string());
        dungeon_repo::insert(db.pool(), &dungeon, "w1")
            .await
            .unwrap();
        let room = Room::new("r1".to_string(), Description::new(None));
        room_repo::insert(db.pool(), &room, "d1").await.unwrap();
        let room2 = Room::new("r2".to_string(), Description::new(None));
        room_repo::insert(db.pool(), &room2, "d1").await.unwrap();
    }

    #[tokio::test]
    async fn upsert_definition_and_find_by_id_round_trip() {
        let db = Database::connect_in_memory().await.unwrap();
        let feature = chest_feature();
        upsert_definition(db.pool(), &feature).await.unwrap();

        let found = find_definition_by_id(db.pool(), "chest")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found, feature);
    }

    #[tokio::test]
    async fn upsert_definition_updates_existing_row() {
        let db = Database::connect_in_memory().await.unwrap();
        let mut feature = chest_feature();
        upsert_definition(db.pool(), &feature).await.unwrap();

        feature.name = "Iron Chest".to_string();
        upsert_definition(db.pool(), &feature).await.unwrap();

        let found = find_definition_by_id(db.pool(), "chest")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.name, "Iron Chest");
    }

    #[tokio::test]
    async fn find_definition_by_id_returns_none_for_missing() {
        let db = Database::connect_in_memory().await.unwrap();
        let found = find_definition_by_id(db.pool(), "missing").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn insert_placement_if_missing_inserts_new_row() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        upsert_definition(db.pool(), &chest_feature())
            .await
            .unwrap();

        let (id, is_new) =
            insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed")
                .await
                .unwrap();
        assert!(is_new);

        let found = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, id);
        assert_eq!(found[0].current_state, "closed");
    }

    #[tokio::test]
    async fn insert_placement_if_missing_is_idempotent() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        upsert_definition(db.pool(), &chest_feature())
            .await
            .unwrap();

        let (id1, is_new1) =
            insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed")
                .await
                .unwrap();
        let (id2, is_new2) =
            insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed")
                .await
                .unwrap();

        assert!(is_new1);
        assert!(!is_new2);
        assert_eq!(id1, id2);
    }

    #[tokio::test]
    async fn insert_placement_if_missing_preserves_modified_state() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        upsert_definition(db.pool(), &chest_feature())
            .await
            .unwrap();

        let (id, _) = insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed")
            .await
            .unwrap();
        update_state(db.pool(), id, "open").await.unwrap();

        insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed")
            .await
            .unwrap();

        let found = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert_eq!(found[0].current_state, "open");
    }

    #[tokio::test]
    async fn find_by_location_is_empty_for_other_rooms() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        upsert_definition(db.pool(), &chest_feature())
            .await
            .unwrap();
        insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed")
            .await
            .unwrap();

        let found = find_by_location(db.pool(), &other_location())
            .await
            .unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn update_state_changes_current_state() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        upsert_definition(db.pool(), &chest_feature())
            .await
            .unwrap();
        let (id, _) = insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed")
            .await
            .unwrap();

        update_state(db.pool(), id, "open").await.unwrap();

        let found = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert_eq!(found[0].current_state, "open");
    }

    #[tokio::test]
    async fn delete_by_room_removes_only_that_room() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        upsert_definition(db.pool(), &chest_feature())
            .await
            .unwrap();
        insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed")
            .await
            .unwrap();
        insert_placement_if_missing(db.pool(), &other_location(), "chest", "closed")
            .await
            .unwrap();

        delete_by_room(db.pool(), "r1").await.unwrap();

        assert!(
            find_by_location(db.pool(), &test_location())
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            find_by_location(db.pool(), &other_location())
                .await
                .unwrap()
                .len(),
            1
        );
    }
}
