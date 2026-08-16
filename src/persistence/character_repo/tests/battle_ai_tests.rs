use super::super::*;
use super::support::{setup, test_location};
use crate::persistence::database::Database;

#[tokio::test]
async fn insert_and_find_preserves_default_battle_ai_type() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let character = Character::new(0, CharacterType::Enemy, test_location());
    let id = insert(db.pool(), &character).await.unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert_eq!(found.battle_ai.ai_type, BattleAiType::None);
}

#[tokio::test]
async fn update_battle_ai_type_persists_simple_random() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let character = Character::new(0, CharacterType::Enemy, test_location());
    let id = insert(db.pool(), &character).await.unwrap();

    update_battle_ai_type(db.pool(), id, &BattleAiType::SimpleRandom)
        .await
        .unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert_eq!(found.battle_ai.ai_type, BattleAiType::SimpleRandom);
}

#[tokio::test]
async fn update_battle_ai_type_can_reset_to_none() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let character = Character::new(0, CharacterType::Enemy, test_location());
    let id = insert(db.pool(), &character).await.unwrap();

    update_battle_ai_type(db.pool(), id, &BattleAiType::SimpleRandom)
        .await
        .unwrap();
    update_battle_ai_type(db.pool(), id, &BattleAiType::None)
        .await
        .unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert_eq!(found.battle_ai.ai_type, BattleAiType::None);
}
