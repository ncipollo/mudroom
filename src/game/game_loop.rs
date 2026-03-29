pub mod attributes;
pub mod effects;
pub mod engagement;
pub mod interactions;
pub mod world_update;

use std::sync::Arc;

use tokio::time::{Duration, interval};

use crate::game::GameState;
use crate::persistence::Database;

pub async fn run(game_state: Arc<GameState>, db: Database) {
    let tick_rate = game_state.mud_config.game_loop.tick_rate;
    let world_update_ms = game_state.mud_config.game_loop.world_update_ms;
    let world_update_ticks = (world_update_ms / tick_rate).max(1);

    let mut ticker = interval(Duration::from_millis(tick_rate));
    let mut tick: u64 = 0;

    loop {
        ticker.tick().await;

        interactions::process(&game_state, &db, tick).await;
        engagement::process(&game_state, tick).await;
        effects::process(&game_state, tick).await;
        attributes::process(&game_state, tick).await;

        if tick.is_multiple_of(world_update_ticks) {
            world_update::process(&game_state, tick).await;
        }

        tick = tick.wrapping_add(1);
    }
}
