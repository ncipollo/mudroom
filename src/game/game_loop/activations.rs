use std::sync::Arc;

use crate::game::GameState;
use crate::game::game_state::PendingActivation;
use crate::game::interaction::room_threats;
use crate::persistence::Database;

pub async fn process(game_state: &Arc<GameState>, db: &Database) {
    let pending = game_state.drain_pending_activations().await;
    for activation in pending {
        apply_activation(game_state, db, activation).await;
    }
}

async fn apply_activation(
    game_state: &Arc<GameState>,
    db: &Database,
    activation: PendingActivation,
) {
    let room_id = activation.entity.location.room_id.clone();

    game_state
        .active_entities
        .write()
        .await
        .insert(activation.entity.id, activation.entity);

    game_state
        .active_players
        .write()
        .await
        .insert(activation.client_id, activation.player.clone());

    if let Err(e) = game_state.sync_active_entities(db.pool()).await {
        tracing::error!(error = %e, "Failed to sync active entities on player activation");
    }

    room_threats::check_room_hostility(game_state, &activation.player, &room_id).await;
}
