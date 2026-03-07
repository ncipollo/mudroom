use std::sync::Arc;

use crate::game::GameState;

pub async fn process(_game_state: &Arc<GameState>, tick: u64) {
    tracing::debug!("Processing attributes tick={tick}");
}
