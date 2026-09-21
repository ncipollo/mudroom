use std::sync::Arc;

use crate::game::GameState;
use crate::game::engagement::battle;

use super::conversation;

/// Process all active engagements for the current game tick: advance every conversation one
/// step via [`Conversations::process_tick`], dispatch resolved actions to
/// [`conversation::handle`] (removing engagements it ends), then advance every battle through
/// its full tick lifecycle via [`battle::process_ticks`].
pub async fn process(game_state: &Arc<GameState>, _tick: u64) {
    let max_engage_ticks = (game_state.mud_config.game_loop.max_engage_ms
        / game_state.mud_config.game_loop.tick_rate_ms)
        .max(1);

    let resolved = game_state
        .engagements
        .conversations
        .process_tick(max_engage_ticks)
        .await;
    for r in &resolved {
        let ended = conversation::handle(game_state, r).await;
        if ended {
            game_state
                .engagements
                .conversations
                .remove(r.engagement_id)
                .await;
        }
    }

    battle::process_ticks(game_state, max_engage_ticks).await;
    battle::run_battle_ai(game_state).await;
}
