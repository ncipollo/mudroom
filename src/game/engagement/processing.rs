use std::sync::Arc;

use crate::game::GameState;
use crate::game::engagement::EngagementType;
use crate::game::engagement::battle;

use super::conversation;

/// Process all active engagements for the current game tick.
///
/// Pipeline per tick:
/// 1. Compute `max_engage_ticks` from the mud config (`max_engage_ms / tick_rate_ms`).
/// 2. [`Engagements::process_tick`] — advance every non-battle engagement one step. Any
///    engagement whose current entity has submitted an action (or whose turn timed out) is
///    resolved and returned as a [`crate::game::ResolvedAction`].
/// 3. Dispatch each resolved action to its type-specific handler:
///    - Conversation: [`conversation::handle`] resolves the player action and returns whether
///      the engagement ended; if so, `processing` removes it from `Engagements`.
/// 4. [`Engagements::tick_battles`] — advance every battle through its phase state machine,
///    returning a [`battle::BattleTick`] snapshot per battle.
/// 5. [`battle::tick::handle_tick`] — for each snapshot, apply innate and queued-ability
///    effects, detect deaths, and broadcast the updated battle state to players. Returns a
///    [`battle::tick::BattleTickOutcome`] so `processing` can update participants, conclude,
///    and remove the engagement without `handle_tick` reaching back into `Engagements`.
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

    let battle_results = game_state.engagements.tick_battles(max_engage_ticks).await;
    for result in battle_results {
        let outcome = battle::tick::handle_tick(game_state, result, max_engage_ticks).await;
        let surviving = game_state
            .engagements
            .update_battle_participants(outcome.engagement_id, &outcome.dead_entity_ids)
            .await;
        if surviving <= 1 {
            battle::tick::handle_battle_ended(game_state, &outcome).await;
            game_state
                .engagements
                .conclude_battle(outcome.engagement_id)
                .await;
            game_state.engagements.remove(outcome.engagement_id).await;
        }
    }
}
