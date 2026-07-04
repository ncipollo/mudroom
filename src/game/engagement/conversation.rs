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
/// - **`Respond { content }`**: the player sent a free-text message to an agent NPC.
///   The handler calls the LLM provider and streams the response.
///
/// Returns `true` if the engagement ended this tick so the caller can call
/// `engagements.remove()` without this module reaching back into `Engagements` directly.
mod agent_turn;
pub mod collection;
pub mod factory;

pub use collection::Conversations;

use std::sync::Arc;

use crate::game::TurnAction;
use crate::game::config::DialogLine;
use crate::game::engagement::ResolvedAction;
use crate::game::interaction::conversation::{format_dialog_message, pick_text};
use crate::game::messaging::ConversationKind;
use crate::game::player::Player;
use crate::game::{GameState, messaging};

use agent_turn::AgentTurnHandler;

/// Returns `true` if the engagement ended and the caller should call `engagements.remove()`.
pub async fn handle(game_state: &Arc<GameState>, resolved: &ResolvedAction) -> bool {
    let Some((player_entity_id, npc_entity_id)) = find_player_and_npc(game_state, resolved).await
    else {
        return false;
    };
    let Some(player) = find_player_by_entity(game_state, player_entity_id).await else {
        return false;
    };
    dispatch_action(game_state, &player, npc_entity_id, resolved).await
}

async fn dispatch_action(
    game_state: &Arc<GameState>,
    player: &Player,
    npc_entity_id: i64,
    resolved: &ResolvedAction,
) -> bool {
    match &resolved.action {
        None => handle_timeout(game_state, player, npc_entity_id, resolved.engagement_id).await,
        Some(TurnAction::SelectDialogChoice { choice }) => {
            handle_choice(
                game_state,
                player,
                npc_entity_id,
                resolved.engagement_id,
                choice,
            )
            .await
        }
        Some(TurnAction::Respond { content }) => {
            let handler =
                AgentTurnHandler::new(game_state, player, npc_entity_id, resolved.engagement_id);
            if handler.has_agent_state().await {
                handler.handle(content).await
            } else {
                tracing::warn!(
                    "NPC {npc_entity_id} has no agent state for engagement {}",
                    resolved.engagement_id
                );
                false
            }
        }
        Some(_) => false,
    }
}

/// Returns `true` — the engagement should be removed by the caller.
async fn handle_timeout(
    game_state: &Arc<GameState>,
    player: &Player,
    npc_entity_id: i64,
    engagement_id: i64,
) -> bool {
    messaging::message(
        &game_state.message_tx,
        player.id,
        "The conversation has ended.",
    );
    remove_npc_conversation_state(game_state, npc_entity_id, engagement_id).await;
    messaging::conversation_ended(&game_state.message_tx, player.id);
    true
}

/// Returns `true` if the engagement ended (invalid choice falls through to no-op → `false`).
async fn handle_choice(
    game_state: &Arc<GameState>,
    player: &Player,
    npc_entity_id: i64,
    engagement_id: i64,
    choice: &str,
) -> bool {
    let index: usize = match choice.parse::<usize>() {
        Ok(n) if n >= 1 => n - 1,
        _ => {
            messaging::message(&game_state.message_tx, player.id, "Invalid choice.");
            resend_current_dialog(game_state, player, npc_entity_id, engagement_id).await;
            return false;
        }
    };

    let dialog = match get_current_dialog(game_state, npc_entity_id, engagement_id).await {
        Some(d) => d,
        None => return true,
    };

    let response = match dialog.responses.get(index) {
        Some(r) => r,
        None => {
            messaging::message(&game_state.message_tx, player.id, "Invalid choice.");
            resend_current_dialog(game_state, player, npc_entity_id, engagement_id).await;
            return false;
        }
    };

    match &response.reply {
        None => end_conversation(game_state, player, npc_entity_id, engagement_id).await,
        Some(reply) => {
            advance_dialog(game_state, player, npc_entity_id, engagement_id, reply).await
        }
    }
}

async fn get_current_dialog(
    game_state: &Arc<GameState>,
    npc_entity_id: i64,
    engagement_id: i64,
) -> Option<DialogLine> {
    let entities = game_state.active_entities.read().await;
    entities
        .get(&npc_entity_id)
        .and_then(|e| e.ai.as_ref())
        .and_then(|ai| ai.simple_conversation_state.as_ref())
        .and_then(|s| s.contexts.get(&engagement_id))
        .and_then(|ctx| ctx.current_dialog.clone())
}

/// Returns `true` — the engagement should be removed by the caller.
async fn end_conversation(
    game_state: &Arc<GameState>,
    player: &Player,
    npc_entity_id: i64,
    engagement_id: i64,
) -> bool {
    remove_npc_conversation_state(game_state, npc_entity_id, engagement_id).await;
    messaging::conversation_ended(&game_state.message_tx, player.id);
    true
}

/// Returns `true` if the dialog tree is exhausted and the engagement should end.
async fn advance_dialog(
    game_state: &Arc<GameState>,
    player: &Player,
    npc_entity_id: i64,
    engagement_id: i64,
    reply: &DialogLine,
) -> bool {
    let reply_text = pick_text(reply).to_string();
    let reply_responses = reply.responses.clone();

    {
        let mut entities = game_state.active_entities.write().await;
        if let Some(npc) = entities.get_mut(&npc_entity_id)
            && let Some(ai) = npc.ai.as_mut()
            && let Some(state) = ai.simple_conversation_state.as_mut()
            && let Some(ctx) = state.contexts.get_mut(&engagement_id)
        {
            ctx.current_dialog = Some(reply.clone());
        }
    }

    let msg = format_dialog_message(&reply_text, &reply_responses);
    messaging::stream_message(game_state.message_tx.clone(), player.id, msg);
    if reply_responses.is_empty() {
        end_conversation(game_state, player, npc_entity_id, engagement_id).await
    } else {
        let options: Vec<String> = reply_responses.iter().map(|r| r.text.clone()).collect();
        messaging::conversation_started(
            &game_state.message_tx,
            player.id,
            ConversationKind::Dialog,
            options,
        );
        false
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
        let options: Vec<String> = d.responses.iter().map(|r| r.text.clone()).collect();
        let text = pick_text(&d).to_string();
        let msg = format_dialog_message(&text, &d.responses);
        messaging::stream_message(game_state.message_tx.clone(), player.id, msg);
        if !options.is_empty() {
            messaging::conversation_started(
                &game_state.message_tx,
                player.id,
                ConversationKind::Dialog,
                options,
            );
        }
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
    {
        if let Some(state) = ai.simple_conversation_state.as_mut() {
            state.contexts.remove(&engagement_id);
        }
        if let Some(state) = ai.agent_conversation_state.as_mut() {
            state.contexts.remove(&engagement_id);
        }
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
