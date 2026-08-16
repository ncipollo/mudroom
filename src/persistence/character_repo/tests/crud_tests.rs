use super::super::*;
use super::support::{setup, test_location};
use crate::game::Room;
use crate::persistence::database::Database;
use crate::persistence::room_repo;

#[tokio::test]
async fn insert_and_find_by_id() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;
    let character = Character::new(0, CharacterType::Player, test_location());
    let id = insert(db.pool(), &character).await.unwrap();

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
async fn find_by_location_returns_characters() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;
    insert(
        db.pool(),
        &Character::new(0, CharacterType::Player, test_location()),
    )
    .await
    .unwrap();
    insert(
        db.pool(),
        &Character::new(0, CharacterType::Character, test_location()),
    )
    .await
    .unwrap();

    let characters = find_by_location(db.pool(), &test_location()).await.unwrap();
    assert_eq!(characters.len(), 2);
}

#[tokio::test]
async fn update_location_changes_location() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;
    // Add a second room for the new location
    let room2 = Room::new("r2".to_string(), Description::new(None));
    room_repo::insert(db.pool(), &room2, "d1").await.unwrap();

    let character = Character::new(0, CharacterType::Player, test_location());
    let id = insert(db.pool(), &character).await.unwrap();

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
async fn delete_removes_character() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;
    let character = Character::new(0, CharacterType::Player, test_location());
    let id = insert(db.pool(), &character).await.unwrap();
    delete(db.pool(), id).await.unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn delete_by_room_removes_all_characters_in_room() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;
    insert(
        db.pool(),
        &Character::new(0, CharacterType::Player, test_location()),
    )
    .await
    .unwrap();
    insert(
        db.pool(),
        &Character::new(0, CharacterType::Character, test_location()),
    )
    .await
    .unwrap();

    delete_by_room(db.pool(), "r1").await.unwrap();

    let characters = find_by_location(db.pool(), &test_location()).await.unwrap();
    assert!(characters.is_empty());
}

#[tokio::test]
async fn insert_and_find_preserves_name() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let mut character = Character::new(0, CharacterType::Player, test_location());
    character.name = "Aragorn".to_string();
    let id = insert(db.pool(), &character).await.unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert_eq!(found.name, "Aragorn");
}

#[tokio::test]
async fn insert_and_find_preserves_attributes() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let mut character = Character::new(0, CharacterType::Character, test_location());
    character.attributes.insert(
        "hp".to_string(),
        Attribute::new("hp".to_string(), 0, 100, 80),
    );
    character.attributes.insert(
        "mp".to_string(),
        Attribute::new("mp".to_string(), 0, 50, 50),
    );
    let id = insert(db.pool(), &character).await.unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert_eq!(found.attributes.len(), 2);
    assert_eq!(found.attributes["hp"].current_value, 80);
    assert_eq!(found.attributes["mp"].max_value, 50);
}

#[tokio::test]
async fn find_by_id_with_corrupt_attributes_returns_empty() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let character = Character::new(0, CharacterType::Character, test_location());
    let id = insert(db.pool(), &character).await.unwrap();

    // Corrupt the attributes column
    sqlx::query("UPDATE characters SET attributes = ? WHERE id = ?")
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

    let character = Character::new(0, CharacterType::Character, test_location());
    let id = insert(db.pool(), &character).await.unwrap();

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
