mod feature;
mod matching;
mod parse;

use std::sync::Arc;

use tracing;

use crate::game::component::{Item, Location};
use crate::game::player::Player;
use crate::game::{GameState, messaging};
use crate::persistence::Database;
use crate::persistence::PersistenceError;
use crate::persistence::{inventory_repo, room_feature_repo, world_loot_repo};
use matching::{TakeMatch, TakeSource, matching_items};
use parse::TakeTarget;

/// Bundles the character state shared by every `take` flow so passing it around doesn't
/// blow up individual functions' argument counts.
#[derive(Clone, Copy)]
struct TakeBudget<'a> {
    inventory_type: &'a str,
    bag_len: usize,
}

pub async fn process(game_state: &Arc<GameState>, db: &Database, player: &Player, target: &str) {
    let Some((location, inventory_type, bag_len)) = character_snapshot(game_state, player).await
    else {
        return;
    };
    let budget = TakeBudget {
        inventory_type: &inventory_type,
        bag_len,
    };

    match parse::parse_take_target(target) {
        TakeTarget::Named(item) => {
            take_named(game_state, db, player, &location, item, budget).await;
        }
        TakeTarget::FromFeature { item, feature } => {
            feature::take_from_feature(game_state, db, player, &location, item, feature, budget)
                .await;
        }
        TakeTarget::AllFromFeature { feature } => {
            feature::take_all_from_feature(game_state, db, player, &location, feature, budget)
                .await;
        }
    }
}

async fn take_named(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    location: &Location,
    target: &str,
    budget: TakeBudget<'_>,
) {
    let matches = match matching_items(game_state, db, location, target).await {
        Ok(matches) => matches,
        Err(e) => {
            tracing::error!("Failed to load takeable items for take: {e}");
            return;
        }
    };
    respond_to_item_matches(game_state, db, player, target, &matches, budget).await;
}

/// Shared 0/1/many dispatch for a set of name-matched items — used by both the room-wide
/// `Named` path and `take <item> from <feature>`, so both read the same "not here" /
/// "which one" wording.
async fn respond_to_item_matches(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    label: &str,
    matches: &[TakeMatch],
    budget: TakeBudget<'_>,
) {
    match matches {
        [] => messaging::message(
            &game_state.message_tx,
            player.id,
            format!("You don't see a '{label}' here."),
        ),
        [item] => take_matched_item(game_state, db, player, item, budget).await,
        _ => messaging::message(
            &game_state.message_tx,
            player.id,
            format!("Which '{label}' do you mean?"),
        ),
    }
}

async fn character_snapshot(
    game_state: &Arc<GameState>,
    player: &Player,
) -> Option<(Location, String, usize)> {
    let characters = game_state.active_characters.read().await;
    let character = characters.get(&player.entity_id)?;
    Some((
        character.location.clone(),
        character.inventory.inventory_type.clone(),
        character.inventory.bag.len(),
    ))
}

async fn take_matched_item(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    item: &TakeMatch,
    budget: TakeBudget<'_>,
) {
    if budget.bag_len >= bag_size_for(game_state, budget.inventory_type) {
        messaging::message(&game_state.message_tx, player.id, "Your bag is full.");
        return;
    }

    let Some(new_item_id) = persist_take(db, player.entity_id, item).await else {
        return;
    };
    add_to_bag(game_state, player, new_item_id, item).await;

    messaging::message(
        &game_state.message_tx,
        player.id,
        format!("You take the {}.", item.name),
    );
}

fn bag_size_for(game_state: &Arc<GameState>, inventory_type: &str) -> usize {
    game_state
        .inventory_config
        .resolve(inventory_type)
        .map(|def| def.bag_size)
        .unwrap_or(usize::MAX)
}

/// Inserts the bag row before removing the item from its source so a persistence failure
/// never causes an item to vanish from the world without landing in the player's bag.
async fn persist_take(db: &Database, character_id: i64, item: &TakeMatch) -> Option<i64> {
    let insert_result =
        inventory_repo::add_bag_item(db.pool(), character_id, &item.item_definition_id).await;
    let new_item_id = log_on_error(insert_result, "add taken item to bag")?;

    match &item.source {
        TakeSource::WorldLoot { loot_id } => {
            let mark_result = world_loot_repo::mark_taken(db.pool(), *loot_id).await;
            log_on_error(mark_result, "mark taken world loot");
        }
        TakeSource::Feature { room_feature_id } => {
            let remove_result = room_feature_repo::remove_item(
                db.pool(),
                *room_feature_id,
                &item.item_definition_id,
            )
            .await;
            log_on_error(remove_result, "remove taken item from feature");
        }
    }

    Some(new_item_id)
}

fn log_on_error<T>(result: Result<T, PersistenceError>, action: &str) -> Option<T> {
    result
        .inspect_err(|e| tracing::error!("Failed to {action}: {e}"))
        .ok()
}

async fn add_to_bag(
    game_state: &Arc<GameState>,
    player: &Player,
    new_item_id: i64,
    item: &TakeMatch,
) {
    let mut characters = game_state.active_characters.write().await;
    if let Some(character) = characters.get_mut(&player.entity_id) {
        character.inventory.bag.push(Item {
            id: new_item_id,
            item_definition_id: item.item_definition_id.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::game::component::description::Description;
    use crate::game::component::{EquippedBonuses, ItemDefinition, ItemUseType};
    use crate::game::entity::character::{Character, CharacterType};
    use crate::game::messaging::Message;
    use crate::game::{Dungeon, FeatureState, Room, RoomFeature, World};
    use crate::persistence::{character_repo, dungeon_repo, item_repo, room_repo, world_repo};

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

    fn chest_feature(items: Vec<String>) -> RoomFeature {
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
            id: "chest".to_string(),
            name: "Oak Chest".to_string(),
            default_state: "open".to_string(),
            states,
            alternate_names: vec![],
        }
    }

    /// Registers a feature holding `def` and places it in `test_location()`. Returns the
    /// placement id.
    async fn seed_feature_item(
        game_state: &Arc<GameState>,
        db: &Database,
        def: ItemDefinition,
    ) -> i64 {
        item_repo::upsert_definition(db.pool(), &def).await.unwrap();
        let feature = chest_feature(vec![def.id.clone()]);
        room_feature_repo::upsert_definition(db.pool(), &feature)
            .await
            .unwrap();
        let (id, _) = room_feature_repo::insert_placement_if_missing(
            db.pool(),
            &test_location(),
            "chest",
            "open",
            std::slice::from_ref(&def.id),
        )
        .await
        .unwrap();
        game_state
            .item_definitions
            .write()
            .await
            .insert(def.id.clone(), def);
        id
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

    #[tokio::test]
    async fn take_takes_item_from_a_room_feature_and_persists_the_removal() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let (game_state, player) = game_state_with_character(&db).await;
        let room_feature_id =
            seed_feature_item(&game_state, &db, definition("torch", "Torch")).await;

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, "torch").await;

        let msg = rx.recv().await.unwrap();
        match msg.message {
            Message::Complete { content, .. } => assert_eq!(content, "You take the Torch."),
            other => panic!("expected Complete message, got {other:?}"),
        }

        let characters = game_state.active_characters.read().await;
        assert_eq!(characters[&player.entity_id].inventory.bag.len(), 1);
        assert_eq!(
            characters[&player.entity_id].inventory.bag[0].item_definition_id,
            "torch"
        );
        drop(characters);

        let placed = room_feature_repo::find_by_location(db.pool(), &test_location())
            .await
            .unwrap();
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].id, room_feature_id);
        assert!(placed[0].items.is_empty());
    }

    #[tokio::test]
    async fn take_is_ambiguous_when_floor_loot_and_feature_item_share_a_name() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let (game_state, player) = game_state_with_character(&db).await;
        seed_loot(&game_state, &db, definition("torch", "Torch")).await;
        seed_feature_item(&game_state, &db, definition("torch", "Torch")).await;

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, "torch").await;

        let msg = rx.recv().await.unwrap();
        match msg.message {
            Message::Complete { content, .. } => {
                assert_eq!(content, "Which 'torch' do you mean?");
            }
            other => panic!("expected Complete message, got {other:?}"),
        }
    }
}
