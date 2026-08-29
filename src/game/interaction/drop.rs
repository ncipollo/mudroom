use std::sync::Arc;

use tracing;

use crate::game::component::{Item, ItemDefinition, Location};
use crate::game::entity::world_loot::WorldLoot;
use crate::game::interaction::inventory;
use crate::game::player::Player;
use crate::game::{GameState, messaging};
use crate::persistence::Database;
use crate::persistence::PersistenceError;
use crate::persistence::{inventory_repo, world_loot_repo};

enum ItemSource {
    Bag,
    Equipped(String),
}

struct DropSnapshot {
    location: Location,
    item: Item,
    source: ItemSource,
}

pub async fn process(game_state: &Arc<GameState>, db: &Database, player: &Player, item_id: i64) {
    let Some(snapshot) = drop_snapshot(game_state, player, item_id).await else {
        messaging::message(
            &game_state.message_tx,
            player.id,
            "You don't have that item.",
        );
        return;
    };

    let definitions = game_state.item_definitions.read().await;
    let Some(definition) = definitions.get(&snapshot.item.item_definition_id).cloned() else {
        return;
    };
    drop(definitions);

    persist_drop(db, &snapshot, &definition).await;
    apply_drop_in_memory(game_state, player, &snapshot).await;

    messaging::message(
        &game_state.message_tx,
        player.id,
        format!("You drop the {}.", definition.name),
    );
    inventory::process(game_state, player).await;
}

async fn drop_snapshot(
    game_state: &Arc<GameState>,
    player: &Player,
    item_id: i64,
) -> Option<DropSnapshot> {
    let characters = game_state.active_characters.read().await;
    let character = characters.get(&player.entity_id)?;
    if let Some(item) = character.inventory.bag.iter().find(|i| i.id == item_id) {
        return Some(DropSnapshot {
            location: character.location.clone(),
            item: item.clone(),
            source: ItemSource::Bag,
        });
    }
    let (slot_name, item) = character
        .inventory
        .equipment
        .iter()
        .find(|(_, i)| i.id == item_id)
        .map(|(slot_name, i)| (slot_name.clone(), i.clone()))?;
    Some(DropSnapshot {
        location: character.location.clone(),
        item,
        source: ItemSource::Equipped(slot_name),
    })
}

/// Inserts the world-loot row before removing the inventory row, mirroring `take.rs`'s
/// insert-before-mark ordering so a persistence failure never causes an item to vanish from both
/// the character's inventory and the world.
async fn persist_drop(db: &Database, snapshot: &DropSnapshot, definition: &ItemDefinition) {
    let loot = WorldLoot::new(
        0,
        snapshot.item.item_definition_id.clone(),
        snapshot.location.clone(),
        definition.name.clone(),
        definition.description.clone(),
    );
    log_on_error(
        world_loot_repo::insert(db.pool(), &loot).await,
        "drop item into world",
    );
    log_on_error(
        inventory_repo::remove_item(db.pool(), snapshot.item.id).await,
        "remove dropped item",
    );
}

fn log_on_error<T>(result: Result<T, PersistenceError>, action: &str) -> Option<T> {
    result
        .inspect_err(|e| tracing::error!("Failed to {action}: {e}"))
        .ok()
}

async fn apply_drop_in_memory(
    game_state: &Arc<GameState>,
    player: &Player,
    snapshot: &DropSnapshot,
) {
    let mut characters = game_state.active_characters.write().await;
    if let Some(character) = characters.get_mut(&player.entity_id) {
        match &snapshot.source {
            ItemSource::Bag => character.inventory.bag.retain(|i| i.id != snapshot.item.id),
            ItemSource::Equipped(slot_name) => {
                character.inventory.equipment.remove(slot_name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::{Description, EquippedBonuses, ItemUseType};
    use crate::game::entity::character::{Character, CharacterType};
    use crate::game::messaging::Message;
    use crate::game::{Dungeon, Room, World};
    use crate::persistence::{
        character_repo, dungeon_repo, inventory_repo, item_repo, room_repo, world_repo,
    };

    fn test_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    async fn setup(db: &Database) -> i64 {
        world_repo::insert(db.pool(), &World::new("w1".to_string()))
            .await
            .unwrap();
        dungeon_repo::insert(db.pool(), &Dungeon::new("d1".to_string()), "w1")
            .await
            .unwrap();
        let room = Room::new("r1".to_string(), Description::new(None));
        room_repo::insert(db.pool(), &room, "d1").await.unwrap();

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

    fn definition() -> ItemDefinition {
        ItemDefinition {
            id: "spiked_bat".to_string(),
            name: "Spiked Bat".to_string(),
            description: Description::new(Some("A studded bat.".to_string())),
            use_type: ItemUseType::Passive,
            item_type: "weapon".to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects: vec![],
            alternate_names: vec![],
        }
    }

    async fn register_definition(game_state: &Arc<GameState>, db: &Database) {
        item_repo::upsert_definition(db.pool(), &definition())
            .await
            .unwrap();
        game_state
            .item_definitions
            .write()
            .await
            .insert("spiked_bat".to_string(), definition());
    }

    #[tokio::test]
    async fn drop_from_bag_creates_world_loot_and_clears_bag() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        register_definition(&game_state, &db).await;
        let item_id = inventory_repo::add_bag_item(db.pool(), character_id, "spiked_bat")
            .await
            .unwrap();

        let mut character = Character::new(character_id, CharacterType::Player, test_location());
        character.inventory.bag.push(Item {
            id: item_id,
            item_definition_id: "spiked_bat".to_string(),
        });
        game_state
            .active_characters
            .write()
            .await
            .insert(character_id, character);
        let player = test_player(character_id);

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, item_id).await;

        let characters = game_state.active_characters.read().await;
        assert!(characters[&character_id].inventory.bag.is_empty());
        drop(characters);

        let loot = world_loot_repo::find_by_location(db.pool(), &test_location())
            .await
            .unwrap();
        assert_eq!(loot.len(), 1);
        assert_eq!(loot[0].item_definition_id, "spiked_bat");

        let bag = inventory_repo::find_bag_by_character(db.pool(), character_id)
            .await
            .unwrap();
        assert!(bag.is_empty());

        let msg = rx.recv().await.unwrap();
        match msg.message {
            Message::Complete { content, .. } => assert_eq!(content, "You drop the Spiked Bat."),
            other => panic!("expected Complete message, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn drop_from_equipped_slot_creates_world_loot_and_clears_slot() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        register_definition(&game_state, &db).await;
        let item_id = inventory_repo::add_bag_item(db.pool(), character_id, "spiked_bat")
            .await
            .unwrap();
        inventory_repo::equip_item(db.pool(), item_id, "weapon")
            .await
            .unwrap();

        let mut character = Character::new(character_id, CharacterType::Player, test_location());
        character.inventory.equipment.insert(
            "weapon".to_string(),
            Item {
                id: item_id,
                item_definition_id: "spiked_bat".to_string(),
            },
        );
        game_state
            .active_characters
            .write()
            .await
            .insert(character_id, character);
        let player = test_player(character_id);

        process(&game_state, &db, &player, item_id).await;

        let characters = game_state.active_characters.read().await;
        assert!(characters[&character_id].inventory.equipment.is_empty());
        drop(characters);

        let loot = world_loot_repo::find_by_location(db.pool(), &test_location())
            .await
            .unwrap();
        assert_eq!(loot.len(), 1);
    }

    #[tokio::test]
    async fn drop_rejects_missing_item() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        game_state.active_characters.write().await.insert(
            character_id,
            Character::new(character_id, CharacterType::Player, test_location()),
        );
        let player = test_player(character_id);

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, 999).await;

        let msg = rx.recv().await.unwrap();
        match msg.message {
            Message::Complete { content, .. } => assert_eq!(content, "You don't have that item."),
            other => panic!("expected Complete message, got {other:?}"),
        }
    }
}
