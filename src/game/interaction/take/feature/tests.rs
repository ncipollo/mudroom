use std::collections::HashMap;

use super::*;
use crate::game::component::description::Description;
use crate::game::component::{EquippedBonuses, Item, ItemDefinition, ItemUseType};
use crate::game::entity::character::{Character, CharacterType};
use crate::game::messaging::Message;
use crate::game::{Dungeon, FeatureState, Room, RoomFeature, World};
use crate::persistence::{
    character_repo, dungeon_repo, item_repo, room_repo, world_loot_repo, world_repo,
};

fn test_location() -> Location {
    Location {
        world_id: "w1".to_string(),
        dungeon_id: "d1".to_string(),
        room_id: "r1".to_string(),
    }
}

fn definition(id: &str, name: &str) -> ItemDefinition {
    ItemDefinition {
        id: id.to_string(),
        name: name.to_string(),
        description: Description::new(None),
        use_type: ItemUseType::Passive,
        item_type: "weapon".to_string(),
        equipped_bonuses: EquippedBonuses::default(),
        use_effects: vec![],
        alternate_names: vec![],
    }
}

async fn setup_world(db: &Database) {
    world_repo::insert(db.pool(), &World::new("w1".to_string()))
        .await
        .unwrap();
    dungeon_repo::insert(db.pool(), &Dungeon::new("d1".to_string()), "w1")
        .await
        .unwrap();
    room_repo::insert(
        db.pool(),
        &Room::new("r1".to_string(), Description::new(None)),
        "d1",
    )
    .await
    .unwrap();
}

async fn setup_character(db: &Database) -> i64 {
    let character = Character::new(0, CharacterType::Player, test_location());
    character_repo::insert(db.pool(), &character).await.unwrap()
}

fn test_player(entity_id: i64) -> Player {
    Player {
        id: 1,
        client_id: "client".to_string(),
        name: "Hero".to_string(),
        entity_id,
    }
}

async fn game_state_with_character(db: &Database) -> (Arc<GameState>, Player) {
    let character_id = setup_character(db).await;
    let game_state = Arc::new(GameState::load(None).unwrap());
    game_state.active_characters.write().await.insert(
        character_id,
        Character::new(character_id, CharacterType::Player, test_location()),
    );
    (game_state, test_player(character_id))
}

fn chest_feature(id: &str, name: &str, items: Vec<String>) -> RoomFeature {
    let mut states = HashMap::new();
    states.insert(
        "open".to_string(),
        FeatureState {
            description: Description::new(None),
            items,
            interact_script: None,
            interact_next_state: None,
            alt_verbs: vec![],
        },
    );
    RoomFeature {
        id: id.to_string(),
        name: name.to_string(),
        default_state: "open".to_string(),
        states,
        alternate_names: vec![],
    }
}

/// Registers `feature` (already carrying its item ids) and places it in
/// `test_location()`. Also registers each item definition and seeds it into
/// `game_state.item_definitions`.
async fn seed_feature(
    game_state: &Arc<GameState>,
    db: &Database,
    feature: &RoomFeature,
    item_defs: Vec<ItemDefinition>,
) -> i64 {
    room_feature_repo::upsert_definition(db.pool(), feature)
        .await
        .unwrap();
    let items = feature.states["open"].items.clone();
    let (id, _) = room_feature_repo::insert_placement_if_missing(
        db.pool(),
        &test_location(),
        &feature.id,
        "open",
        &items,
    )
    .await
    .unwrap();
    for def in item_defs {
        item_repo::upsert_definition(db.pool(), &def).await.unwrap();
        game_state
            .item_definitions
            .write()
            .await
            .insert(def.id.clone(), def);
    }
    id
}

async fn seed_loot(game_state: &Arc<GameState>, db: &Database, def: ItemDefinition) {
    item_repo::upsert_definition(db.pool(), &def).await.unwrap();
    world_loot_repo::insert_config_loot_if_missing(db.pool(), &test_location(), &def.id)
        .await
        .unwrap();
    game_state
        .item_definitions
        .write()
        .await
        .insert(def.id.clone(), def);
}

async fn recv_message(
    rx: &mut tokio::sync::broadcast::Receiver<crate::game::messaging::PlayerMessage>,
) -> String {
    let msg = rx.recv().await.unwrap();
    match msg.message {
        Message::Complete { content, .. } => content,
        other => panic!("expected Complete message, got {other:?}"),
    }
}

#[tokio::test]
async fn take_from_feature_takes_only_from_that_feature() {
    let db = Database::connect_in_memory().await.unwrap();
    setup_world(&db).await;
    let (game_state, player) = game_state_with_character(&db).await;
    seed_loot(&game_state, &db, definition("torch", "Torch")).await;
    seed_feature(
        &game_state,
        &db,
        &chest_feature("chest", "Oak Chest", vec!["torch".to_string()]),
        vec![],
    )
    .await;

    let mut rx = game_state.message_tx.subscribe();
    super::super::process(&game_state, &db, &player, "torch from oak chest").await;
    let content = recv_message(&mut rx).await;

    assert_eq!(content, "You take the Torch.");
    let loot = world_loot_repo::find_by_location(db.pool(), &test_location())
        .await
        .unwrap();
    assert_eq!(loot.len(), 1, "floor loot should be untouched");
}

#[tokio::test]
async fn take_from_feature_reports_no_match_when_item_not_held_by_that_feature() {
    let db = Database::connect_in_memory().await.unwrap();
    setup_world(&db).await;
    let (game_state, player) = game_state_with_character(&db).await;
    seed_loot(&game_state, &db, definition("torch", "Torch")).await;
    seed_feature(
        &game_state,
        &db,
        &chest_feature("chest", "Oak Chest", vec!["key".to_string()]),
        vec![definition("key", "Key")],
    )
    .await;

    let mut rx = game_state.message_tx.subscribe();
    super::super::process(&game_state, &db, &player, "torch from oak chest").await;
    let content = recv_message(&mut rx).await;

    assert_eq!(content, "You don't see a 'torch' here.");
}

#[tokio::test]
async fn take_from_feature_reports_not_found_for_unknown_feature_name() {
    let db = Database::connect_in_memory().await.unwrap();
    setup_world(&db).await;
    let (game_state, player) = game_state_with_character(&db).await;

    let mut rx = game_state.message_tx.subscribe();
    super::super::process(&game_state, &db, &player, "torch from oak chest").await;
    let content = recv_message(&mut rx).await;

    assert_eq!(content, "You don't see a 'oak chest' here.");
}

#[tokio::test]
async fn take_from_feature_reports_ambiguous_when_two_features_share_a_name() {
    let db = Database::connect_in_memory().await.unwrap();
    setup_world(&db).await;
    let (game_state, player) = game_state_with_character(&db).await;
    seed_feature(
        &game_state,
        &db,
        &chest_feature("chest", "Oak Chest", vec![]),
        vec![],
    )
    .await;
    seed_feature(
        &game_state,
        &db,
        &chest_feature("chest_2", "Oak Chest", vec![]),
        vec![],
    )
    .await;

    let mut rx = game_state.message_tx.subscribe();
    super::super::process(&game_state, &db, &player, "torch from oak chest").await;
    let content = recv_message(&mut rx).await;

    assert_eq!(content, "Which 'oak chest' do you mean?");
}

#[tokio::test]
async fn take_from_feature_matches_by_alternate_name_case_insensitively() {
    let db = Database::connect_in_memory().await.unwrap();
    setup_world(&db).await;
    let (game_state, player) = game_state_with_character(&db).await;
    let mut feature = chest_feature("chest", "Oak Chest", vec!["torch".to_string()]);
    feature.alternate_names = vec!["chest".to_string()];
    seed_feature(
        &game_state,
        &db,
        &feature,
        vec![definition("torch", "Torch")],
    )
    .await;

    let mut rx = game_state.message_tx.subscribe();
    super::super::process(&game_state, &db, &player, "torch from CHEST").await;
    let content = recv_message(&mut rx).await;

    assert_eq!(content, "You take the Torch.");
}

#[tokio::test]
async fn take_all_from_feature_matches_by_alternate_name() {
    let db = Database::connect_in_memory().await.unwrap();
    setup_world(&db).await;
    let (game_state, player) = game_state_with_character(&db).await;
    let mut feature = chest_feature(
        "chest",
        "Oak Chest",
        vec!["torch".to_string(), "key".to_string()],
    );
    feature.alternate_names = vec!["chest".to_string()];
    seed_feature(
        &game_state,
        &db,
        &feature,
        vec![definition("torch", "Torch"), definition("key", "Key")],
    )
    .await;

    let mut rx = game_state.message_tx.subscribe();
    super::super::process(&game_state, &db, &player, "all from chest").await;
    let content = recv_message(&mut rx).await;

    assert_eq!(
        content,
        "You take the Torch and the Key from the Oak Chest."
    );
}

#[tokio::test]
async fn take_from_feature_primary_name_wins_over_alias_match() {
    let db = Database::connect_in_memory().await.unwrap();
    setup_world(&db).await;
    let (game_state, player) = game_state_with_character(&db).await;
    seed_feature(
        &game_state,
        &db,
        &chest_feature("club", "Club", vec!["torch".to_string()]),
        vec![definition("torch", "Torch")],
    )
    .await;
    let mut spiked_bat = chest_feature("spiked_bat", "Spiked Bat", vec!["key".to_string()]);
    spiked_bat.alternate_names = vec!["club".to_string()];
    seed_feature(
        &game_state,
        &db,
        &spiked_bat,
        vec![definition("key", "Key")],
    )
    .await;

    let mut rx = game_state.message_tx.subscribe();
    super::super::process(&game_state, &db, &player, "torch from club").await;
    let content = recv_message(&mut rx).await;

    assert_eq!(content, "You take the Torch.");
}

#[tokio::test]
async fn take_all_from_feature_takes_every_item() {
    let db = Database::connect_in_memory().await.unwrap();
    setup_world(&db).await;
    let (game_state, player) = game_state_with_character(&db).await;
    seed_feature(
        &game_state,
        &db,
        &chest_feature(
            "chest",
            "Oak Chest",
            vec!["torch".to_string(), "key".to_string()],
        ),
        vec![definition("torch", "Torch"), definition("key", "Key")],
    )
    .await;

    let mut rx = game_state.message_tx.subscribe();
    super::super::process(&game_state, &db, &player, "all from oak chest").await;
    let content = recv_message(&mut rx).await;

    assert_eq!(
        content,
        "You take the Torch and the Key from the Oak Chest."
    );
    let characters = game_state.active_characters.read().await;
    assert_eq!(characters[&player.entity_id].inventory.bag.len(), 2);
}

#[tokio::test]
async fn take_all_from_feature_stops_when_bag_becomes_full() {
    let db = Database::connect_in_memory().await.unwrap();
    setup_world(&db).await;
    let (game_state, player) = game_state_with_character(&db).await;
    seed_feature(
        &game_state,
        &db,
        &chest_feature(
            "chest",
            "Oak Chest",
            vec!["torch".to_string(), "key".to_string()],
        ),
        vec![definition("torch", "Torch"), definition("key", "Key")],
    )
    .await;
    {
        let mut characters = game_state.active_characters.write().await;
        let character = characters.get_mut(&player.entity_id).unwrap();
        // default inventory config's bag_size is 20 — fill 19 slots so only one of the
        // chest's two items fits.
        for i in 0..19 {
            character.inventory.bag.push(Item {
                id: 1000 + i,
                item_definition_id: "torch".to_string(),
            });
        }
    }

    let mut rx = game_state.message_tx.subscribe();
    super::super::process(&game_state, &db, &player, "all from oak chest").await;
    let content = recv_message(&mut rx).await;

    assert_eq!(
        content,
        "You take the Torch from the Oak Chest. Your bag is full, so you leave the rest."
    );
    let characters = game_state.active_characters.read().await;
    assert_eq!(characters[&player.entity_id].inventory.bag.len(), 20);
}

#[tokio::test]
async fn take_all_from_feature_reports_full_bag_when_already_full() {
    let db = Database::connect_in_memory().await.unwrap();
    setup_world(&db).await;
    let (game_state, player) = game_state_with_character(&db).await;
    seed_feature(
        &game_state,
        &db,
        &chest_feature("chest", "Oak Chest", vec!["torch".to_string()]),
        vec![definition("torch", "Torch")],
    )
    .await;
    {
        let mut characters = game_state.active_characters.write().await;
        let character = characters.get_mut(&player.entity_id).unwrap();
        for i in 0..20 {
            character.inventory.bag.push(Item {
                id: 1000 + i,
                item_definition_id: "torch".to_string(),
            });
        }
    }

    let mut rx = game_state.message_tx.subscribe();
    super::super::process(&game_state, &db, &player, "all from oak chest").await;
    let content = recv_message(&mut rx).await;

    assert_eq!(content, "Your bag is full.");
}

#[tokio::test]
async fn take_all_from_feature_when_feature_holds_nothing() {
    let db = Database::connect_in_memory().await.unwrap();
    setup_world(&db).await;
    let (game_state, player) = game_state_with_character(&db).await;
    seed_feature(
        &game_state,
        &db,
        &chest_feature("chest", "Oak Chest", vec![]),
        vec![],
    )
    .await;

    let mut rx = game_state.message_tx.subscribe();
    super::super::process(&game_state, &db, &player, "all from oak chest").await;
    let content = recv_message(&mut rx).await;

    assert_eq!(content, "There's nothing to take from the Oak Chest.");
}
