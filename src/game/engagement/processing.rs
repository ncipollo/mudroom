use std::sync::Arc;

use crate::game::GameState;
use crate::game::engagement::EngagementType;
use crate::game::engagement::battle;

use super::conversation;

/// Process all active engagements for the current game tick.
///
/// Pipeline per tick:
/// 1. Compute `max_engage_ticks` from the mud config (`max_engage_ms / tick_rate_ms`).
/// 2. [`Engagements::process_tick`] — advance every non-battle engagement one step and
///    return resolved actions for entities whose turn completed or timed out.
/// 3. Dispatch each resolved action to its type-specific handler:
///    - Conversation: [`conversation::handle`] returns whether the engagement ended;
///      if so, `processing` removes it from `Engagements`.
/// 4. [`battle::process_ticks`] — advance every battle through its full tick lifecycle
///    (phase state machine → effect resolution → dead-entity removal → conclusion).
pub async fn process(game_state: &Arc<GameState>, _tick: u64) {
    let max_engage_ticks = (game_state.mud_config.game_loop.max_engage_ms
        / game_state.mud_config.game_loop.tick_rate_ms)
        .max(1);

    let resolved = game_state.engagements.process_tick(max_engage_ticks).await;
    for r in &resolved {
        if r.engagement_type == EngagementType::Conversation {
            let ended = conversation::handle(game_state, r).await;
            if ended {
                game_state.engagements.remove(r.engagement_id).await;
            }
        }
    }

    battle::process_ticks(game_state, max_engage_ticks).await;
    battle::run_battle_ai(game_state).await;
}
