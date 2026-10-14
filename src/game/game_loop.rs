pub mod activations;
pub mod attributes;
pub mod effects;
pub mod interactions;
pub mod world_update;

use std::sync::Arc;

use tokio::time::{Duration, interval};

use crate::game::GameState;
use crate::game::engagement;
use crate::persistence::Database;

pub async fn run(game_state: Arc<GameState>, db: Database) {
    let tick_rate = game_state.mud_config.game_loop.tick_rate_ms;

    let mut ticker = interval(Duration::from_millis(tick_rate));

    loop {
        ticker.tick().await;
        run_once(&game_state, &db).await;
    }
}

/// Runs exactly one tick of the game loop's processing sequence, out of band from the
/// real-time ticker. Shared by `run` and the debug tick-advance endpoint so both paths stay
/// on the same tick counter (see `GameState::next_tick`).
pub async fn run_once(game_state: &Arc<GameState>, db: &Database) -> u64 {
    let tick = game_state.next_tick();

    activations::process(game_state, db).await;
    interactions::process(game_state, db, tick).await;
    engagement::process(game_state, tick).await;
    effects::process(game_state, tick).await;
    attributes::process(game_state, tick).await;
    world_update::process(game_state, db, tick).await;

    tick
}
