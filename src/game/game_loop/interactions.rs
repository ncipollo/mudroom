use std::sync::Arc;

use crate::game::{GameState, interaction};
use crate::persistence::Database;

pub async fn process(game_state: &Arc<GameState>, db: &Database, tick: u64) {
    interaction::process(game_state, db, tick).await;
}
