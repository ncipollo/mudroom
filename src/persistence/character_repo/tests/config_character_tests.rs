use super::super::*;
use super::support::{setup, test_location};
use crate::persistence::database::Database;

#[tokio::test]
async fn find_config_characters_by_dungeon_returns_matching() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let (id, _) = insert_config_character_if_missing(
        db.pool(),
        &CharacterType::Character,
        &test_location(),
        "entities/innkeeper",
        None,
        "innkeeper",
    )
    .await
    .unwrap();

    // Player character (no config_id) should not be returned
    insert(
        db.pool(),
        &Character::new(0, CharacterType::Player, test_location()),
    )
    .await
    .unwrap();

    let found = find_config_characters_by_dungeon(db.pool(), "w1", "d1")
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, id);

    // Different dungeon returns nothing
    let other = find_config_characters_by_dungeon(db.pool(), "w1", "d2")
        .await
        .unwrap();
    assert!(other.is_empty());
}

#[tokio::test]
async fn insert_config_character_if_missing_stores_name() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let (id, _) = insert_config_character_if_missing(
        db.pool(),
        &CharacterType::Character,
        &test_location(),
        "entities/innkeeper",
        None,
        "innkeeper",
    )
    .await
    .unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert_eq!(found.name, "innkeeper");
}

#[tokio::test]
async fn insert_config_character_if_missing_updates_name_on_conflict() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let (id, _) = insert_config_character_if_missing(
        db.pool(),
        &CharacterType::Character,
        &test_location(),
        "entities/innkeeper",
        None,
        "old name",
    )
    .await
    .unwrap();

    insert_config_character_if_missing(
        db.pool(),
        &CharacterType::Character,
        &test_location(),
        "entities/innkeeper",
        None,
        "innkeeper",
    )
    .await
    .unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert_eq!(found.name, "innkeeper");
}

#[tokio::test]
async fn insert_config_character_if_missing_stores_description() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let (id, _) = insert_config_character_if_missing(
        db.pool(),
        &CharacterType::Character,
        &test_location(),
        "entities/innkeeper",
        Some("A friendly innkeeper."),
        "innkeeper",
    )
    .await
    .unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert_eq!(
        found.description.text.as_deref(),
        Some("A friendly innkeeper.")
    );
}

#[tokio::test]
async fn insert_config_character_if_missing_updates_description_on_conflict() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let (id, _) = insert_config_character_if_missing(
        db.pool(),
        &CharacterType::Character,
        &test_location(),
        "entities/innkeeper",
        Some("Old description."),
        "innkeeper",
    )
    .await
    .unwrap();

    insert_config_character_if_missing(
        db.pool(),
        &CharacterType::Character,
        &test_location(),
        "entities/innkeeper",
        Some("New description."),
        "innkeeper",
    )
    .await
    .unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert_eq!(found.description.text.as_deref(), Some("New description."));
}

#[tokio::test]
async fn insert_config_character_if_missing_inserts_new() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let (id, is_new) = insert_config_character_if_missing(
        db.pool(),
        &CharacterType::Character,
        &test_location(),
        "entities/innkeeper",
        None,
        "innkeeper",
    )
    .await
    .unwrap();
    assert!(id > 0);
    assert!(is_new);

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert_eq!(found.config_id.as_deref(), Some("entities/innkeeper"));
}

#[tokio::test]
async fn insert_config_character_if_missing_returns_existing_id() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let (id1, is_new1) = insert_config_character_if_missing(
        db.pool(),
        &CharacterType::Character,
        &test_location(),
        "entities/innkeeper",
        None,
        "innkeeper",
    )
    .await
    .unwrap();
    let (id2, is_new2) = insert_config_character_if_missing(
        db.pool(),
        &CharacterType::Character,
        &test_location(),
        "entities/innkeeper",
        None,
        "innkeeper",
    )
    .await
    .unwrap();
    assert_eq!(id1, id2);
    assert!(is_new1);
    assert!(!is_new2);

    let characters = find_by_location(db.pool(), &test_location()).await.unwrap();
    assert_eq!(characters.len(), 1);
}
