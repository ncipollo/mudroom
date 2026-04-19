/// Handles resolved actions for [`crate::game::EngagementType::Conversation`] engagements.
///
/// A conversation engagement has two entities: a player and an NPC. Only the player takes
/// turns; the NPC's entity id is tracked for dialog-state lookups but never appears in the
/// turn order.
///
/// Each time the player's turn resolves there are two possible outcomes:
/// - **Timeout** (`resolved.action` is `None`): the player didn't respond in time.
///   The conversation is ended, the NPC's conversation state is cleaned up, and the
///   engagement is removed.
/// - **`SelectDialogChoice { choice }`**: the player picked a numbered dialog option.
///   The handler validates the choice, advances to the matching reply node in the dialog
///   tree, updates the NPC's in-memory conversation context, and sends the next dialog
///   message to the player. If the reply has no further responses the conversation ends.
use std::sync::Arc;

use crate::game::TurnAction;
use crate::game::engagement::ResolvedAction;
use crate::game::game_loop::interactions::conversation::{format_dialog_message, pick_text};
use crate::game::player::Player;
use crate::game::{GameState, messaging};

pub async fn handle(game_state: &Arc<GameState>, resolved: &ResolvedAction) {
    let (player_entity_id, npc_entity_id) = match find_player_and_npc(game_state, resolved).await {
        Some(pair) => pair,
        None => return,
    };

    let player = match find_player_by_entity(game_state, player_entity_id).await {
        Some(p) => p,
        None => return,
    };

    match &resolved.action {
        None => {
            // Player's turn timed out — end the conversation
            messaging::message(
                &game_state.message_tx,
                player.id,
                "The conversation has ended.",
            );
            remove_npc_conversation_state(game_state, npc_entity_id, resolved.engagement_id).await;
            game_state.engagements.remove(resolved.engagement_id).await;
        }
        Some(TurnAction::SelectDialogChoice { choice }) => {
            handle_choice(
                game_state,
                &player,
                npc_entity_id,
                resolved.engagement_id,
                choice,
            )
            .await;
        }
        Some(_) => {
            // Other action types are not handled in conversation engagements
        }
    }
}

async fn handle_choice(
    game_state: &Arc<GameState>,
    player: &Player,
    npc_entity_id: i64,
    engagement_id: i64,
    choice: &str,
) {
    let index: usize = match choice.parse::<usize>() {
        Ok(n) if n >= 1 => n - 1,
        _ => {
            messaging::message(&game_state.message_tx, player.id, "Invalid choice.");
            resend_current_dialog(game_state, player, npc_entity_id, engagement_id).await;
            return;
        }
    };

    let current_dialog = {
        let entities = game_state.active_entities.read().await;
        entities
            .get(&npc_entity_id)
            .and_then(|e| e.ai.as_ref())
            .and_then(|ai| ai.simple_conversation_state.as_ref())
            .and_then(|s| s.contexts.get(&engagement_id))
            .and_then(|ctx| ctx.current_dialog.clone())
    };

    let dialog = match current_dialog {
        Some(d) => d,
        None => {
            game_state.engagements.remove(engagement_id).await;
            return;
        }
    };

    let response = match dialog.responses.get(index) {
        Some(r) => r,
        None => {
            messaging::message(&game_state.message_tx, player.id, "Invalid choice.");
            resend_current_dialog(game_state, player, npc_entity_id, engagement_id).await;
            return;
        }
    };

    match &response.reply {
        None => {
            // End of dialog tree
            remove_npc_conversation_state(game_state, npc_entity_id, engagement_id).await;
            game_state.engagements.remove(engagement_id).await;
        }
        Some(reply) => {
            let reply_text = pick_text(reply).to_string();
            let reply_responses = reply.responses.clone();

            {
                let mut entities = game_state.active_entities.write().await;
                if let Some(npc) = entities.get_mut(&npc_entity_id)
                    && let Some(ai) = npc.ai.as_mut()
                    && let Some(state) = ai.simple_conversation_state.as_mut()
                    && let Some(ctx) = state.contexts.get_mut(&engagement_id)
                {
                    ctx.current_dialog = Some(*reply.clone());
                }
            }

            let msg = format_dialog_message(&reply_text, &reply_responses);
            messaging::stream_message(game_state.message_tx.clone(), player.id, msg);
        }
    }
}

async fn resend_current_dialog(
    game_state: &Arc<GameState>,
    player: &Player,
    npc_entity_id: i64,
    engagement_id: i64,
) {
    let dialog = {
        let entities = game_state.active_entities.read().await;
        entities
            .get(&npc_entity_id)
            .and_then(|e| e.ai.as_ref())
            .and_then(|ai| ai.simple_conversation_state.as_ref())
            .and_then(|s| s.contexts.get(&engagement_id))
            .and_then(|ctx| ctx.current_dialog.clone())
    };
    if let Some(d) = dialog {
        let text = pick_text(&d).to_string();
        let msg = format_dialog_message(&text, &d.responses);
        messaging::stream_message(game_state.message_tx.clone(), player.id, msg);
    }
}

async fn remove_npc_conversation_state(
    game_state: &Arc<GameState>,
    npc_entity_id: i64,
    engagement_id: i64,
) {
    let mut entities = game_state.active_entities.write().await;
    if let Some(npc) = entities.get_mut(&npc_entity_id)
        && let Some(ai) = npc.ai.as_mut()
        && let Some(state) = ai.simple_conversation_state.as_mut()
    {
        state.contexts.remove(&engagement_id);
    }
}

/// Determine which entity is the player and which is the NPC.
/// The player is the one whose turn resolved (entity_id in the resolved action).
async fn find_player_and_npc(
    game_state: &Arc<GameState>,
    resolved: &ResolvedAction,
) -> Option<(i64, i64)> {
    let player_entity_id = resolved.entity_id;
    let npc_entity_id = resolved
        .entity_ids
        .iter()
        .find(|&&id| id != player_entity_id)
        .copied()?;
    // Verify the player entity actually has an active player record
    let players = game_state.active_players.read().await;
    let is_player = players.values().any(|p| p.entity_id == player_entity_id);
    if is_player {
        Some((player_entity_id, npc_entity_id))
    } else {
        None
    }
}

async fn find_player_by_entity(game_state: &Arc<GameState>, entity_id: i64) -> Option<Player> {
    game_state
        .active_players
        .read()
        .await
        .values()
        .find(|p| p.entity_id == entity_id)
        .cloned()
}
