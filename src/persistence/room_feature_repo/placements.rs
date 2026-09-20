use sqlx::SqlitePool;

use crate::game::RoomFeatureState;
use crate::game::component::Location;
use crate::persistence::error::PersistenceError;

type RoomFeatureRow = (i64, String, String, String, String, String, String);

/// Insert a placement if none exists yet for this (location, feature) pair. Returns
/// `(id, is_new)`. An existing row is left untouched — its `current_state` and `items` may
/// have been changed at runtime and must survive a resync.
pub async fn insert_placement_if_missing(
    pool: &SqlitePool,
    location: &Location,
    feature_definition_id: &str,
    default_state: &str,
    items: &[String],
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

    let items_json = serde_json::to_string(items)?;
    let result = sqlx::query(
        "INSERT INTO room_features \
             (feature_definition_id, world_id, dungeon_id, room_id, current_state, items_json) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(feature_definition_id)
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .bind(default_state)
    .bind(items_json)
    .execute(pool)
    .await?;

    Ok((result.last_insert_rowid(), true))
}

pub async fn find_by_location(
    pool: &SqlitePool,
    location: &Location,
) -> Result<Vec<RoomFeatureState>, PersistenceError> {
    let rows: Vec<RoomFeatureRow> = sqlx::query_as(
        "SELECT id, feature_definition_id, world_id, dungeon_id, room_id, current_state, items_json \
         FROM room_features WHERE world_id = ? AND dungeon_id = ? AND room_id = ?",
    )
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(parse_room_feature_state).collect()
}

fn parse_room_feature_state(row: RoomFeatureRow) -> Result<RoomFeatureState, PersistenceError> {
    let (id, feature_definition_id, world_id, dungeon_id, room_id, current_state, items_json) = row;
    let items: Vec<String> = serde_json::from_str(&items_json)?;
    Ok(RoomFeatureState::new(
        id,
        feature_definition_id,
        Location {
            world_id,
            dungeon_id,
            room_id,
        },
        current_state,
        items,
    ))
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

/// Removes the first occurrence of `item_definition_id` from a room feature's current
/// item holdings. Returns `true` if an item was removed. Duplicate ids in a feature's
/// items are interchangeable copies, so which occurrence is removed doesn't matter.
pub async fn remove_item(
    pool: &SqlitePool,
    id: i64,
    item_definition_id: &str,
) -> Result<bool, PersistenceError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT items_json FROM room_features WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    let Some((items_json,)) = row else {
        return Ok(false);
    };

    let mut items: Vec<String> = serde_json::from_str(&items_json)?;
    let Some(position) = items.iter().position(|item| item == item_definition_id) else {
        return Ok(false);
    };
    items.remove(position);

    let updated_json = serde_json::to_string(&items)?;
    sqlx::query("UPDATE room_features SET items_json = ? WHERE id = ?")
        .bind(updated_json)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(true)
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
    use std::collections::HashMap;

    use super::*;
    use crate::game::{Description, Dungeon, FeatureState, Room, RoomFeature, World};
    use crate::persistence::database::Database;
    use crate::persistence::room_feature_repo::upsert_definition;
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
                description: Description::new(Some("A closed chest.".to_string())),
                items: vec![],
                interact_script: None,
                interact_next_state: Some("open".to_string()),
            },
        );
        states.insert(
            "open".to_string(),
            FeatureState {
                description: Description::new(Some("An open chest.".to_string())),
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
            alt_verbs: vec![],
            alternate_names: vec![],
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
    async fn insert_placement_if_missing_inserts_new_row() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        upsert_definition(db.pool(), &chest_feature())
            .await
            .unwrap();

        let (id, is_new) =
            insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed", &[])
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
            insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed", &[])
                .await
                .unwrap();
        let (id2, is_new2) =
            insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed", &[])
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

        let (id, _) =
            insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed", &[])
                .await
                .unwrap();
        update_state(db.pool(), id, "open").await.unwrap();

        insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed", &[])
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
        insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed", &[])
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
        let (id, _) =
            insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed", &[])
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
        insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed", &[])
            .await
            .unwrap();
        insert_placement_if_missing(db.pool(), &other_location(), "chest", "closed", &[])
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

    #[tokio::test]
    async fn insert_placement_if_missing_seeds_items() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        upsert_definition(db.pool(), &chest_feature())
            .await
            .unwrap();

        insert_placement_if_missing(
            db.pool(),
            &test_location(),
            "chest",
            "open",
            &["medicine".to_string()],
        )
        .await
        .unwrap();

        let found = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert_eq!(found[0].items, vec!["medicine".to_string()]);
    }

    #[tokio::test]
    async fn insert_placement_if_missing_preserves_modified_items_on_resync() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        upsert_definition(db.pool(), &chest_feature())
            .await
            .unwrap();

        let (id, _) = insert_placement_if_missing(
            db.pool(),
            &test_location(),
            "chest",
            "open",
            &["medicine".to_string()],
        )
        .await
        .unwrap();
        remove_item(db.pool(), id, "medicine").await.unwrap();

        insert_placement_if_missing(
            db.pool(),
            &test_location(),
            "chest",
            "open",
            &["medicine".to_string()],
        )
        .await
        .unwrap();

        let found = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert!(found[0].items.is_empty());
    }

    #[tokio::test]
    async fn remove_item_removes_first_matching_occurrence() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        upsert_definition(db.pool(), &chest_feature())
            .await
            .unwrap();
        let (id, _) = insert_placement_if_missing(
            db.pool(),
            &test_location(),
            "chest",
            "open",
            &["medicine".to_string(), "medicine".to_string()],
        )
        .await
        .unwrap();

        let removed = remove_item(db.pool(), id, "medicine").await.unwrap();

        assert!(removed);
        let found = find_by_location(db.pool(), &test_location()).await.unwrap();
        assert_eq!(found[0].items, vec!["medicine".to_string()]);
    }

    #[tokio::test]
    async fn remove_item_returns_false_when_item_not_present() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;
        upsert_definition(db.pool(), &chest_feature())
            .await
            .unwrap();
        let (id, _) =
            insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed", &[])
                .await
                .unwrap();

        let removed = remove_item(db.pool(), id, "medicine").await.unwrap();

        assert!(!removed);
    }

    #[tokio::test]
    async fn remove_item_returns_false_for_missing_feature() {
        let db = Database::connect_in_memory().await.unwrap();
        setup(&db).await;

        let removed = remove_item(db.pool(), 999, "medicine").await.unwrap();

        assert!(!removed);
    }
}
