use std::sync::Arc;

use crate::game::GameState;

pub async fn process(game_state: &Arc<GameState>, _tick: u64) {
    let max_engage_ticks = (game_state.mud_config.game_loop.max_engage_ms
        / game_state.mud_config.game_loop.tick_rate)
        .max(1);
    game_state.engagements.process_tick(max_engage_ticks).await;
}
