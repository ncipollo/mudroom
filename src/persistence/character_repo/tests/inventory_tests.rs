use super::super::*;
use super::support::{setup, test_location};
use crate::game::config::DEFAULT_INVENTORY_TYPE;
use crate::persistence::database::Database;

fn leather_vest_definition() -> crate::game::component::ItemDefinition {
    use crate::game::component::{EquippedBonuses, ItemDefinition, ItemUseType};

    ItemDefinition {
        id: "leather_vest".to_string(),
        name: "Leather Vest".to_string(),
        description: Description::default(),
        use_type: ItemUseType::Passive,
        item_type: "armor".to_string(),
        equipped_bonuses: EquippedBonuses::default(),
        use_effects: vec![],
    }
}

#[tokio::test]
async fn find_by_id_loads_equipped_inventory() {
    use crate::persistence::{inventory_repo, item_repo};

    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let character = Character::new(0, CharacterType::Player, test_location());
    let id = insert(db.pool(), &character).await.unwrap();

    item_repo::upsert_definition(db.pool(), &leather_vest_definition())
        .await
        .unwrap();
    inventory_repo::set_character_equipment(db.pool(), id, &[("armor", "leather_vest")])
        .await
        .unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert_eq!(found.inventory.equipment.len(), 1);
    assert_eq!(
        found.inventory.equipment["armor"].item_definition_id,
        "leather_vest"
    );
}

#[tokio::test]
async fn find_by_id_loads_bag_items() {
    use crate::persistence::{inventory_repo, item_repo};

    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let character = Character::new(0, CharacterType::Player, test_location());
    let id = insert(db.pool(), &character).await.unwrap();

    item_repo::upsert_definition(db.pool(), &leather_vest_definition())
        .await
        .unwrap();
    inventory_repo::set_character_bag_items(db.pool(), id, &["leather_vest"])
        .await
        .unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert_eq!(found.inventory.bag.len(), 1);
    assert_eq!(found.inventory.bag[0].item_definition_id, "leather_vest");
}

#[tokio::test]
async fn find_by_id_defaults_inventory_type_when_no_row_exists() {
    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let character = Character::new(0, CharacterType::Player, test_location());
    let id = insert(db.pool(), &character).await.unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert_eq!(found.inventory.inventory_type, DEFAULT_INVENTORY_TYPE);
}

#[tokio::test]
async fn find_by_id_loads_stored_inventory_type() {
    use crate::persistence::inventory_repo;

    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let character = Character::new(0, CharacterType::Player, test_location());
    let id = insert(db.pool(), &character).await.unwrap();

    inventory_repo::ensure_exists(db.pool(), id, "merchant_stall")
        .await
        .unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert_eq!(found.inventory.inventory_type, "merchant_stall");
}
