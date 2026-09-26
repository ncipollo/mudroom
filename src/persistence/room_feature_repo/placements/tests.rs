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
            alt_verbs: vec![],
        },
    );
    states.insert(
        "open".to_string(),
        FeatureState {
            description: Description::new(Some("An open chest.".to_string())),
            items: vec!["medicine".to_string()],
            interact_script: None,
            interact_next_state: None,
            alt_verbs: vec![],
        },
    );
    RoomFeature {
        id: "chest".to_string(),
        name: "Oak Chest".to_string(),
        default_state: "closed".to_string(),
        states,
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

    let (id, _) = insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed", &[])
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
    let (id, _) = insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed", &[])
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
    let (id, _) = insert_placement_if_missing(db.pool(), &test_location(), "chest", "closed", &[])
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

#[tokio::test]
async fn reset_placement_overwrites_modified_state_and_items() {
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
    update_state(db.pool(), id, "open").await.unwrap();

    reset_placement(
        db.pool(),
        &test_location(),
        "chest",
        "closed",
        &["medicine".to_string()],
    )
    .await
    .unwrap();

    let found = find_by_location(db.pool(), &test_location()).await.unwrap();
    assert_eq!(found[0].current_state, "closed");
    assert_eq!(found[0].items, vec!["medicine".to_string()]);
}

#[tokio::test]
async fn reset_placement_is_noop_for_missing_placement() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;
    upsert_definition(db.pool(), &chest_feature())
        .await
        .unwrap();

    reset_placement(db.pool(), &test_location(), "chest", "closed", &[])
        .await
        .unwrap();

    let found = find_by_location(db.pool(), &test_location()).await.unwrap();
    assert!(found.is_empty());
}
