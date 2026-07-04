use std::sync::Arc;

use crate::game::GameState;
use crate::game::engagement::battle;

use super::conversation;

/// Process all active engagements for the current game tick.
///
/// Pipeline per tick:
/// 1. Compute `max_engage_ticks` from the mud config (`max_engage_ms / tick_rate_ms`).
/// 2. [`Conversations::process_tick`] — advance every conversation engagement one step and
///    return resolved actions for entities whose turn completed or timed out.
/// 3. Dispatch each resolved action to [`conversation::handle`]; if it returns `true`
///    the engagement ended and is removed from `conversations`.
/// 4. [`battle::process_ticks`] — advance every battle through its full tick lifecycle
///    (phase state machine → effect resolution → dead-entity removal → conclusion).
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
