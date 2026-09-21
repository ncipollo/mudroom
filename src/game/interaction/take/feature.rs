mod summary;

use std::sync::Arc;

use tracing;

use crate::game::component::Location;
use crate::game::map::universe::room_feature;
use crate::game::player::Player;
use crate::game::{GameState, messaging};
use crate::persistence::Database;
use crate::persistence::PersistenceError;
use crate::persistence::room_feature_repo;

use super::TakeBudget;
use super::matching::{self, TakeMatch};

/// A room feature that matched a `take ... from <feature>` target by name.
struct FeatureMatch {
    room_feature_id: i64,
    name: String,
    items: Vec<String>,
}

async fn resolve_feature(
    db: &Database,
    location: &Location,
    feature_name: &str,
) -> Result<Vec<FeatureMatch>, PersistenceError> {
    let placements = room_feature_repo::find_by_location(db.pool(), location).await?;
    let mut candidates = Vec::new();
    for placement in placements {
        let Some(def) =
            room_feature_repo::find_definition_by_id(db.pool(), &placement.feature_definition_id)
                .await?
        else {
            continue;
        };
        candidates.push((placement, def));
    }

    Ok(
        room_feature::select_by_name(candidates, feature_name, |c| &c.1)
            .into_iter()
            .map(|(placement, def)| FeatureMatch {
                room_feature_id: placement.id,
                name: def.name,
                items: placement.items,
            })
            .collect(),
    )
}

/// Resolves `feature_name` in `location`, messaging the player and returning `None` on a
/// DB error, no match, or an ambiguous match — mirroring plain `take`'s not-found/ambiguous
/// wording so feature-name resolution reads the same as item-name resolution.
async fn resolve_feature_or_report(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    location: &Location,
    feature_name: &str,
) -> Option<FeatureMatch> {
    let matches = match resolve_feature(db, location, feature_name).await {
        Ok(matches) => matches,
        Err(e) => {
            tracing::error!("Failed to load room features for take: {e}");
            return None;
        }
    };

    match matches.len() {
        0 => {
            messaging::message(
                &game_state.message_tx,
                player.id,
                format!("You don't see a '{feature_name}' here."),
            );
            None
        }
        1 => matches.into_iter().next(),
        _ => {
            messaging::message(
                &game_state.message_tx,
                player.id,
                format!("Which '{feature_name}' do you mean?"),
            );
            None
        }
    }
}

/// `take <item> from <feature>` — matches `item` only against the resolved feature's held
/// items, ignoring floor loot and other features.
pub(super) async fn take_from_feature(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    location: &Location,
    item: &str,
    feature_name: &str,
    budget: TakeBudget<'_>,
) {
    let Some(feature) =
        resolve_feature_or_report(game_state, db, player, location, feature_name).await
    else {
        return;
    };

    let matches = matching::matching_items_in_feature(
        game_state,
        feature.room_feature_id,
        &feature.items,
        item,
    )
    .await;
    super::respond_to_item_matches(game_state, db, player, item, &matches, budget).await;
}

/// `take all from <feature>` — takes every item the resolved feature currently holds,
/// respecting bag-size limits and reporting what was and wasn't taken.
pub(super) async fn take_all_from_feature(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    location: &Location,
    feature_name: &str,
    budget: TakeBudget<'_>,
) {
    let Some(feature) =
        resolve_feature_or_report(game_state, db, player, location, feature_name).await
    else {
        return;
    };

    if feature.items.is_empty() {
        messaging::message(
            &game_state.message_tx,
            player.id,
            format!("There's nothing to take from the {}.", feature.name),
        );
        return;
    }

    let candidates =
        matching::feature_items_as_matches(game_state, feature.room_feature_id, &feature.items)
            .await;
    let message = take_matches(game_state, db, player, &feature.name, &candidates, budget).await;
    messaging::message(&game_state.message_tx, player.id, message);
}

async fn take_matches(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    feature_name: &str,
    candidates: &[TakeMatch],
    budget: TakeBudget<'_>,
) -> String {
    let bag_size = super::bag_size_for(game_state, budget.inventory_type);
    let mut taken_names = Vec::new();
    let mut bag_filled_up = false;
    let mut current_len = budget.bag_len;

    for item in candidates {
        if current_len >= bag_size {
            bag_filled_up = true;
            break;
        }
        let Some(new_item_id) = super::persist_take(db, player.entity_id, item).await else {
            continue;
        };
        super::add_to_bag(game_state, player, new_item_id, item).await;
        taken_names.push(item.name.clone());
        current_len += 1;
    }

    summary::take_all_summary(feature_name, &taken_names, bag_filled_up)
}

#[cfg(test)]
mod tests;
