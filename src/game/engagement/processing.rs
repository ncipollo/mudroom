use std::sync::Arc;

use crate::game::EngagementType;
use crate::game::GameState;

use super::conversation;

/// Process all active engagements for the current game tick.
///
/// Each tick, the engagement system:
/// 1. Calculates the maximum number of ticks a single turn may last before it times out,
///    based on `max_engage_ms / tick_rate_ms` from the mud config.
/// 2. Calls [`crate::game::Engagements::process_tick`] to advance every engagement — any
///    engagement whose current entity has submitted an action (or whose turn has timed out)
///    is resolved and returned as a [`crate::game::ResolvedAction`].
/// 3. Dispatches each resolved action to the appropriate handler based on its
///    [`EngagementType`]. Currently only [`EngagementType::Conversation`] is handled.
pub async fn process(game_state: &Arc<GameState>, _tick: u64) {
    // Derive the per-turn tick budget. Always at least 1 tick so engagements can't stall.
    let max_engage_ticks = (game_state.mud_config.game_loop.max_engage_ms
        / game_state.mud_config.game_loop.tick_rate_ms)
        .max(1);

    // Advance all engagements and collect the ones whose turn just resolved.
    let resolved = game_state.engagements.process_tick(max_engage_ticks).await;

    // Dispatch each resolved action to the right handler.
    for r in &resolved {
        if r.engagement_type == EngagementType::Conversation {
            conversation::handle(game_state, r).await;
        }
    }
}
