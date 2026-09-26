use sqlx::SqlitePool;

use crate::game::RoomFeatureState;
use crate::game::component::Location;
use crate::persistence::error::PersistenceError;

type RoomFeatureRow = (i64, String, String, String, String, String, String);

/// Insert a placement if none exists yet for this (location, feature) pair. Returns
/// `(id, is_new)`. An existing row is left untouched — its `current_state` and `items` may
/// have been changed at runtime and must survive a resync. See [`reset_placement`] for the
/// forced-overwrite counterpart used when an operator explicitly asks for a reset.
pub async fn insert_placement_if_missing(
    pool: &SqlitePool,
    location: &Location,
    feature_definition_id: &str,
    default_state: &str,
    items: &[String],
) -> Result<(i64, bool), PersistenceError> {
    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM room_features \
         WHERE world_id = ? AND dungeon_id = ? AND room_id = ? AND feature_definition_id = ?",
    )
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .bind(feature_definition_id)
    .fetch_optional(pool)
    .await?;

    if let Some((id,)) = existing {
        return Ok((id, false));
    }

    let items_json = serde_json::to_string(items)?;
    let result = sqlx::query(
        "INSERT INTO room_features \
             (feature_definition_id, world_id, dungeon_id, room_id, current_state, items_json) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(feature_definition_id)
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .bind(default_state)
    .bind(items_json)
    .execute(pool)
    .await?;

    Ok((result.last_insert_rowid(), true))
}

pub async fn find_by_location(
    pool: &SqlitePool,
    location: &Location,
) -> Result<Vec<RoomFeatureState>, PersistenceError> {
    let rows: Vec<RoomFeatureRow> = sqlx::query_as(
        "SELECT id, feature_definition_id, world_id, dungeon_id, room_id, current_state, items_json \
         FROM room_features WHERE world_id = ? AND dungeon_id = ? AND room_id = ?",
    )
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(parse_room_feature_state).collect()
}

fn parse_room_feature_state(row: RoomFeatureRow) -> Result<RoomFeatureState, PersistenceError> {
    let (id, feature_definition_id, world_id, dungeon_id, room_id, current_state, items_json) = row;
    let items: Vec<String> = serde_json::from_str(&items_json)?;
    Ok(RoomFeatureState::new(
        id,
        feature_definition_id,
        Location {
            world_id,
            dungeon_id,
            room_id,
        },
        current_state,
        items,
    ))
}

pub async fn update_state(
    pool: &SqlitePool,
    id: i64,
    new_state: &str,
) -> Result<(), PersistenceError> {
    sqlx::query("UPDATE room_features SET current_state = ? WHERE id = ?")
        .bind(new_state)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Removes the first occurrence of `item_definition_id` from a room feature's current
/// item holdings. Returns `true` if an item was removed. Duplicate ids in a feature's
/// items are interchangeable copies, so which occurrence is removed doesn't matter.
pub async fn remove_item(
    pool: &SqlitePool,
    id: i64,
    item_definition_id: &str,
) -> Result<bool, PersistenceError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT items_json FROM room_features WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    let Some((items_json,)) = row else {
        return Ok(false);
    };

    let mut items: Vec<String> = serde_json::from_str(&items_json)?;
    let Some(position) = items.iter().position(|item| item == item_definition_id) else {
        return Ok(false);
    };
    items.remove(position);

    let updated_json = serde_json::to_string(&items)?;
    sqlx::query("UPDATE room_features SET items_json = ? WHERE id = ?")
        .bind(updated_json)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(true)
}

pub async fn delete_by_room(pool: &SqlitePool, room_id: &str) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM room_features WHERE room_id = ?")
        .bind(room_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Forces an existing placement's `current_state` and `items` back to the feature's default,
/// clobbering runtime changes — unlike [`insert_placement_if_missing`]. Both are reset
/// together since a `FeatureState`'s items only make sense alongside that state. No-op if the
/// placement doesn't exist. A single UPDATE, so `items_json` can't end up malformed, but it
/// can still race with a concurrent [`remove_item`] the same way two takes already can.
pub async fn reset_placement(
    pool: &SqlitePool,
    location: &Location,
    feature_definition_id: &str,
    default_state: &str,
    items: &[String],
) -> Result<(), PersistenceError> {
    let items_json = serde_json::to_string(items)?;
    sqlx::query(
        "UPDATE room_features SET current_state = ?, items_json = ? \
         WHERE world_id = ? AND dungeon_id = ? AND room_id = ? AND feature_definition_id = ?",
    )
    .bind(default_state)
    .bind(items_json)
    .bind(&location.world_id)
    .bind(&location.dungeon_id)
    .bind(&location.room_id)
    .bind(feature_definition_id)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests;
