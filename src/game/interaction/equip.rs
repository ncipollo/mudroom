use std::collections::HashMap;
use std::sync::Arc;

use tracing;

use crate::game::component::Item;
use crate::game::interaction::inventory;
use crate::game::player::Player;
use crate::game::{GameState, messaging};
use crate::persistence::Database;
use crate::persistence::PersistenceError;
use crate::persistence::inventory_repo;

struct EquipSnapshot {
    item: Item,
    equipment: HashMap<String, Item>,
    inventory_type: String,
}

pub async fn process(game_state: &Arc<GameState>, db: &Database, player: &Player, item_id: i64) {
    if game_state
        .engagements
        .battles
        .find_for_entity(player.entity_id)
        .await
        .is_some()
    {
        messaging::message(
            &game_state.message_tx,
            player.id,
            "You can't change equipment during battle.",
        );
        return;
    }

    let Some(snapshot) = equip_snapshot(game_state, player, item_id).await else {
        messaging::message(
            &game_state.message_tx,
            player.id,
            "You don't have that item.",
        );
        return;
    };

    equip_matched_item(game_state, db, player, snapshot).await;
}

async fn equip_snapshot(
    game_state: &Arc<GameState>,
    player: &Player,
    item_id: i64,
) -> Option<EquipSnapshot> {
    let characters = game_state.active_characters.read().await;
    let character = characters.get(&player.entity_id)?;
    let item = character
        .inventory
        .bag
        .iter()
        .find(|i| i.id == item_id)?
        .clone();
    Some(EquipSnapshot {
        item,
        equipment: character.inventory.equipment.clone(),
        inventory_type: character.inventory.inventory_type.clone(),
    })
}

async fn equip_matched_item(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    snapshot: EquipSnapshot,
) {
    let definitions = game_state.item_definitions.read().await;
    let Some(definition) = definitions.get(&snapshot.item.item_definition_id).cloned() else {
        return;
    };
    let Some(resolved) = game_state
        .inventory_config
        .resolve(&snapshot.inventory_type)
    else {
        return;
    };
    let Some(slot_name) = resolved
        .eligible_slots(&definition.item_type)
        .first()
        .map(|slot| slot.name.clone())
    else {
        drop(definitions);
        messaging::message(
            &game_state.message_tx,
            player.id,
            format!("The {} can't be equipped.", definition.name),
        );
        return;
    };
    let occupied = snapshot.equipment.get(&slot_name).cloned();
    let occupied_name = occupied
        .as_ref()
        .and_then(|old| definitions.get(&old.item_definition_id))
        .map(|d| d.name.clone());
    drop(definitions);

    persist_equip(db, snapshot.item.id, &slot_name, occupied.as_ref()).await;
    apply_equip_in_memory(game_state, player, &snapshot.item, &slot_name, occupied).await;

    let message = match occupied_name {
        Some(old_name) => format!(
            "You equip the {}, replacing the {old_name}.",
            definition.name
        ),
        None => format!("You equip the {}.", definition.name),
    };
    messaging::message(&game_state.message_tx, player.id, message);
    inventory::process(game_state, player).await;
}

async fn persist_equip(db: &Database, item_id: i64, slot_name: &str, occupied: Option<&Item>) {
    if let Some(old) = occupied {
        log_on_error(
            inventory_repo::unequip_item(db.pool(), old.id).await,
            "unequip item during swap",
        );
    }
    log_on_error(
        inventory_repo::equip_item(db.pool(), item_id, slot_name).await,
        "equip item",
    );
}

fn log_on_error<T>(result: Result<T, PersistenceError>, action: &str) -> Option<T> {
    result
        .inspect_err(|e| tracing::error!("Failed to {action}: {e}"))
        .ok()
}

async fn apply_equip_in_memory(
    game_state: &Arc<GameState>,
    player: &Player,
    item: &Item,
    slot_name: &str,
    occupied: Option<Item>,
) {
    let mut characters = game_state.active_characters.write().await;
    if let Some(character) = characters.get_mut(&player.entity_id) {
        character.inventory.bag.retain(|i| i.id != item.id);
        if let Some(old) = occupied {
            character.inventory.bag.push(old);
        }
        character
            .inventory
            .equipment
            .insert(slot_name.to_string(), item.clone());
    }
}

struct UnequipSnapshot {
    slot_name: String,
    item: Item,
    bag_len: usize,
    inventory_type: String,
}

pub async fn unequip(game_state: &Arc<GameState>, db: &Database, player: &Player, item_id: i64) {
    let Some(snapshot) = unequip_snapshot(game_state, player, item_id).await else {
        messaging::message(
            &game_state.message_tx,
            player.id,
            "You don't have that equipped.",
        );
        return;
    };

    if snapshot.bag_len >= bag_size_for(game_state, &snapshot.inventory_type) {
        messaging::message(&game_state.message_tx, player.id, "Your bag is full.");
        return;
    }

    log_on_error(
        inventory_repo::unequip_item(db.pool(), item_id).await,
        "unequip item",
    );
    apply_unequip_in_memory(
        game_state,
        player,
        &snapshot.slot_name,
        snapshot.item.clone(),
    )
    .await;

    let name = item_name(game_state, &snapshot.item).await;
    messaging::message(
        &game_state.message_tx,
        player.id,
        format!("You unequip the {name}."),
    );
    inventory::process(game_state, player).await;
}

async fn unequip_snapshot(
    game_state: &Arc<GameState>,
    player: &Player,
    item_id: i64,
) -> Option<UnequipSnapshot> {
    let characters = game_state.active_characters.read().await;
    let character = characters.get(&player.entity_id)?;
    let (slot_name, item) = character
        .inventory
        .equipment
        .iter()
        .find(|(_, i)| i.id == item_id)
        .map(|(slot_name, i)| (slot_name.clone(), i.clone()))?;
    Some(UnequipSnapshot {
        slot_name,
        item,
        bag_len: character.inventory.bag.len(),
        inventory_type: character.inventory.inventory_type.clone(),
    })
}

async fn apply_unequip_in_memory(
    game_state: &Arc<GameState>,
    player: &Player,
    slot_name: &str,
    item: Item,
) {
    let mut characters = game_state.active_characters.write().await;
    if let Some(character) = characters.get_mut(&player.entity_id) {
        character.inventory.equipment.remove(slot_name);
        character.inventory.bag.push(item);
    }
}

fn bag_size_for(game_state: &Arc<GameState>, inventory_type: &str) -> usize {
    game_state
        .inventory_config
        .resolve(inventory_type)
        .map(|def| def.bag_size)
        .unwrap_or(usize::MAX)
}

async fn item_name(game_state: &Arc<GameState>, item: &Item) -> String {
    game_state
        .item_definitions
        .read()
        .await
        .get(&item.item_definition_id)
        .map(|d| d.name.clone())
        .unwrap_or_else(|| "item".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::{
        Description, EquippedBonuses, ItemDefinition, ItemUseType, Location,
    };
    use crate::game::entity::character::{Character, CharacterType};
    use crate::game::messaging::Message;
    use crate::game::{Dungeon, Room, World};
    use crate::persistence::{
        character_repo, dungeon_repo, inventory_repo, item_repo, room_repo, world_repo,
    };
    use std::collections::HashMap;

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

    fn item_definition(id: &str, name: &str, item_type: &str) -> ItemDefinition {
        ItemDefinition {
            id: id.to_string(),
            name: name.to_string(),
            description: Description::default(),
            use_type: ItemUseType::Passive,
            item_type: item_type.to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects: vec![],
            alternate_names: vec![],
        }
    }

    async fn register_definition(
        game_state: &Arc<GameState>,
        db: &Database,
        definition: ItemDefinition,
    ) {
        item_repo::upsert_definition(db.pool(), &definition)
            .await
            .unwrap();
        game_state
            .item_definitions
            .write()
            .await
            .insert(definition.id.clone(), definition);
    }

    async fn insert_character(game_state: &Arc<GameState>, character_id: i64) {
        game_state.active_characters.write().await.insert(
            character_id,
            Character::new(character_id, CharacterType::Player, test_location()),
        );
    }

    #[tokio::test]
    async fn equip_moves_item_from_bag_into_empty_slot() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        register_definition(
            &game_state,
            &db,
            item_definition("spiked_bat", "Spiked Bat", "weapon"),
        )
        .await;
        let item_id = inventory_repo::add_bag_item(db.pool(), character_id, "spiked_bat")
            .await
            .unwrap();
        insert_character(&game_state, character_id).await;
        {
            let mut characters = game_state.active_characters.write().await;
            characters
                .get_mut(&character_id)
                .unwrap()
                .inventory
                .bag
                .push(Item {
                    id: item_id,
                    item_definition_id: "spiked_bat".to_string(),
                });
        }
        let player = test_player(character_id);

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, item_id).await;

        let characters = game_state.active_characters.read().await;
        let character = &characters[&character_id];
        assert!(character.inventory.bag.is_empty());
        assert_eq!(character.inventory.equipment["weapon"].id, item_id);
        drop(characters);

        let equipped = inventory_repo::find_equipped_by_character(db.pool(), character_id)
            .await
            .unwrap();
        assert_eq!(equipped["weapon"].id, item_id);

        let msg = rx.recv().await.unwrap();
        match msg.message {
            Message::Complete { content, .. } => assert_eq!(content, "You equip the Spiked Bat."),
            other => panic!("expected Complete message, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn equip_swaps_occupied_slot() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        register_definition(
            &game_state,
            &db,
            item_definition("spiked_bat", "Spiked Bat", "weapon"),
        )
        .await;
        register_definition(
            &game_state,
            &db,
            item_definition("dagger", "Dagger", "weapon"),
        )
        .await;

        let old_item_id = inventory_repo::add_bag_item(db.pool(), character_id, "dagger")
            .await
            .unwrap();
        inventory_repo::equip_item(db.pool(), old_item_id, "weapon")
            .await
            .unwrap();
        let new_item_id = inventory_repo::add_bag_item(db.pool(), character_id, "spiked_bat")
            .await
            .unwrap();

        insert_character(&game_state, character_id).await;
        {
            let mut characters = game_state.active_characters.write().await;
            let character = characters.get_mut(&character_id).unwrap();
            character.inventory.equipment.insert(
                "weapon".to_string(),
                Item {
                    id: old_item_id,
                    item_definition_id: "dagger".to_string(),
                },
            );
            character.inventory.bag.push(Item {
                id: new_item_id,
                item_definition_id: "spiked_bat".to_string(),
            });
        }
        let player = test_player(character_id);

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, new_item_id).await;

        let characters = game_state.active_characters.read().await;
        let character = &characters[&character_id];
        assert_eq!(character.inventory.equipment["weapon"].id, new_item_id);
        assert_eq!(character.inventory.bag.len(), 1);
        assert_eq!(character.inventory.bag[0].id, old_item_id);
        drop(characters);

        let msg = rx.recv().await.unwrap();
        match msg.message {
            Message::Complete { content, .. } => {
                assert_eq!(content, "You equip the Spiked Bat, replacing the Dagger.");
            }
            other => panic!("expected Complete message, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn equip_rejects_item_with_no_eligible_slot() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        register_definition(
            &game_state,
            &db,
            item_definition("trinket", "Trinket", "junk"),
        )
        .await;
        let item_id = inventory_repo::add_bag_item(db.pool(), character_id, "trinket")
            .await
            .unwrap();
        insert_character(&game_state, character_id).await;
        {
            let mut characters = game_state.active_characters.write().await;
            characters
                .get_mut(&character_id)
                .unwrap()
                .inventory
                .bag
                .push(Item {
                    id: item_id,
                    item_definition_id: "trinket".to_string(),
                });
        }
        let player = test_player(character_id);

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, item_id).await;

        let msg = rx.recv().await.unwrap();
        match msg.message {
            Message::Complete { content, .. } => {
                assert_eq!(content, "The Trinket can't be equipped.");
            }
            other => panic!("expected Complete message, got {other:?}"),
        }

        let characters = game_state.active_characters.read().await;
        assert_eq!(characters[&character_id].inventory.bag.len(), 1);
    }

    #[tokio::test]
    async fn equip_rejects_during_battle() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        register_definition(
            &game_state,
            &db,
            item_definition("spiked_bat", "Spiked Bat", "weapon"),
        )
        .await;
        let item_id = inventory_repo::add_bag_item(db.pool(), character_id, "spiked_bat")
            .await
            .unwrap();
        insert_character(&game_state, character_id).await;
        {
            let mut characters = game_state.active_characters.write().await;
            characters
                .get_mut(&character_id)
                .unwrap()
                .inventory
                .bag
                .push(Item {
                    id: item_id,
                    item_definition_id: "spiked_bat".to_string(),
                });
        }
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![character_id]);
        game_state
            .engagements
            .add_battle("r1".to_string(), vec!["player".to_string()], participants)
            .await;
        let player = test_player(character_id);

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, item_id).await;

        let msg = rx.recv().await.unwrap();
        match msg.message {
            Message::Complete { content, .. } => {
                assert_eq!(content, "You can't change equipment during battle.");
            }
            other => panic!("expected Complete message, got {other:?}"),
        }

        let characters = game_state.active_characters.read().await;
        assert_eq!(characters[&character_id].inventory.bag.len(), 1);
    }

    #[tokio::test]
    async fn unequip_moves_item_back_to_bag() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        register_definition(
            &game_state,
            &db,
            item_definition("spiked_bat", "Spiked Bat", "weapon"),
        )
        .await;
        let item_id = inventory_repo::add_bag_item(db.pool(), character_id, "spiked_bat")
            .await
            .unwrap();
        inventory_repo::equip_item(db.pool(), item_id, "weapon")
            .await
            .unwrap();
        insert_character(&game_state, character_id).await;
        {
            let mut characters = game_state.active_characters.write().await;
            characters
                .get_mut(&character_id)
                .unwrap()
                .inventory
                .equipment
                .insert(
                    "weapon".to_string(),
                    Item {
                        id: item_id,
                        item_definition_id: "spiked_bat".to_string(),
                    },
                );
        }
        let player = test_player(character_id);

        let mut rx = game_state.message_tx.subscribe();
        unequip(&game_state, &db, &player, item_id).await;

        let characters = game_state.active_characters.read().await;
        let character = &characters[&character_id];
        assert!(character.inventory.equipment.is_empty());
        assert_eq!(character.inventory.bag.len(), 1);
        assert_eq!(character.inventory.bag[0].id, item_id);
        drop(characters);

        let msg = rx.recv().await.unwrap();
        match msg.message {
            Message::Complete { content, .. } => {
                assert_eq!(content, "You unequip the Spiked Bat.");
            }
            other => panic!("expected Complete message, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn unequip_rejects_when_bag_is_full() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        register_definition(
            &game_state,
            &db,
            item_definition("spiked_bat", "Spiked Bat", "weapon"),
        )
        .await;
        let item_id = inventory_repo::add_bag_item(db.pool(), character_id, "spiked_bat")
            .await
            .unwrap();
        inventory_repo::equip_item(db.pool(), item_id, "weapon")
            .await
            .unwrap();
        insert_character(&game_state, character_id).await;
        {
            let mut characters = game_state.active_characters.write().await;
            let character = characters.get_mut(&character_id).unwrap();
            character.inventory.equipment.insert(
                "weapon".to_string(),
                Item {
                    id: item_id,
                    item_definition_id: "spiked_bat".to_string(),
                },
            );
            // default inventory config's bag_size is 20 — fill it so unequip has nowhere to go.
            for i in 0..20 {
                character.inventory.bag.push(Item {
                    id: 1000 + i,
                    item_definition_id: "spiked_bat".to_string(),
                });
            }
        }
        let player = test_player(character_id);

        let mut rx = game_state.message_tx.subscribe();
        unequip(&game_state, &db, &player, item_id).await;

        let msg = rx.recv().await.unwrap();
        match msg.message {
            Message::Complete { content, .. } => assert_eq!(content, "Your bag is full."),
            other => panic!("expected Complete message, got {other:?}"),
        }

        let characters = game_state.active_characters.read().await;
        assert!(
            characters[&character_id]
                .inventory
                .equipment
                .contains_key("weapon")
        );
    }
}
