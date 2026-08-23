use std::sync::Arc;

use tracing;

use crate::game::component::{Item, Location};
use crate::game::player::Player;
use crate::game::{GameState, messaging};
use crate::persistence::Database;
use crate::persistence::PersistenceError;
use crate::persistence::{inventory_repo, world_loot_repo};

struct LootMatch {
    loot_id: i64,
    item_definition_id: String,
    name: String,
}

pub async fn process(game_state: &Arc<GameState>, db: &Database, player: &Player, target: &str) {
    let Some((location, inventory_type, bag_len)) = character_snapshot(game_state, player).await
    else {
        return;
    };

    let matches = match matching_loot(game_state, db, &location, target).await {
        Ok(matches) => matches,
        Err(e) => {
            tracing::error!("Failed to load world loot for take: {e}");
            return;
        }
    };

    match matches.as_slice() {
        [] => messaging::message(
            &game_state.message_tx,
            player.id,
            format!("You don't see a '{target}' here."),
        ),
        [loot] => take_matched_item(game_state, db, player, loot, &inventory_type, bag_len).await,
        _ => messaging::message(
            &game_state.message_tx,
            player.id,
            format!("Which '{target}' do you mean?"),
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

async fn matching_loot(
    game_state: &Arc<GameState>,
    db: &Database,
    location: &Location,
    target: &str,
) -> Result<Vec<LootMatch>, PersistenceError> {
    let loot = world_loot_repo::find_by_location(db.pool(), location).await?;
    let definitions = game_state.item_definitions.read().await;
    Ok(loot
        .into_iter()
        .filter_map(|l| {
            let def = definitions.get(&l.item_definition_id)?;
            def.name.eq_ignore_ascii_case(target).then(|| LootMatch {
                loot_id: l.id,
                item_definition_id: l.item_definition_id.clone(),
                name: def.name.clone(),
            })
        })
        .collect())
}

async fn take_matched_item(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    loot: &LootMatch,
    inventory_type: &str,
    bag_len: usize,
) {
    if bag_len >= bag_size_for(game_state, inventory_type) {
        messaging::message(&game_state.message_tx, player.id, "Your bag is full.");
        return;
    }

    let Some(new_item_id) = persist_take(db, player.entity_id, loot).await else {
        return;
    };
    add_to_bag(game_state, player, new_item_id, loot).await;

    messaging::message(
        &game_state.message_tx,
        player.id,
        format!("You take the {}.", loot.name),
    );
}

fn bag_size_for(game_state: &Arc<GameState>, inventory_type: &str) -> usize {
    game_state
        .inventory_config
        .resolve(inventory_type)
        .map(|def| def.bag_size)
        .unwrap_or(usize::MAX)
}

/// Inserts the bag row before marking the world-loot row taken so a persistence failure never
/// causes an item to vanish from the world without landing in the player's bag.
async fn persist_take(db: &Database, character_id: i64, loot: &LootMatch) -> Option<i64> {
    let insert_result =
        inventory_repo::add_bag_item(db.pool(), character_id, &loot.item_definition_id).await;
    let new_item_id = log_on_error(insert_result, "add taken item to bag")?;

    let mark_result = world_loot_repo::mark_taken(db.pool(), loot.loot_id).await;
    log_on_error(mark_result, "mark taken world loot");

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
    loot: &LootMatch,
) {
    let mut characters = game_state.active_characters.write().await;
    if let Some(character) = characters.get_mut(&player.entity_id) {
        character.inventory.bag.push(Item {
            id: new_item_id,
            item_definition_id: loot.item_definition_id.clone(),
        });
    }
}
