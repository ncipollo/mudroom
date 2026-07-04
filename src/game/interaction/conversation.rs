mod agent_conversation;

use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::entity_ai::{ConversationContext, EntityAI, SimpleConversationState};
use crate::game::config::{DialogLine, PersonaConfig, PersonaContext, PlayerResponse};
use crate::game::messaging::ConversationKind;
use crate::game::player::Player;
use crate::game::{GameState, messaging};

use agent_conversation::AgentConversationStarter;

enum TalkCandidate {
    Agent {
        npc_entity_id: i64,
        instructions: String,
    },
    StandardDialog {
        npc_entity_id: i64,
        dialog_root: DialogLine,
    },
}

pub async fn end_player_conversation(game_state: &Arc<GameState>, player: &Player) {
    let Some((engagement_id, entity_ids)) = game_state
        .engagements
        .conversations
        .find_for_entity(player.entity_id)
        .await
    else {
        return;
    };
    if let Some(npc_entity_id) = entity_ids
        .iter()
        .copied()
        .find(|&id| id != player.entity_id)
    {
        remove_npc_conversation_state(game_state, npc_entity_id, engagement_id).await;
    }
    game_state
        .engagements
        .conversations
        .remove(engagement_id)
        .await;
    messaging::conversation_ended(&game_state.message_tx, player.id);
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

pub async fn process(
    game_state: &Arc<GameState>,
    player: &Player,
    initial_message: Option<String>,
) {
    if game_state
        .engagements
        .conversations
        .is_entity_in(player.entity_id)
        .await
    {
        messaging::message(
            &game_state.message_tx,
            player.id,
            "You're already in a conversation.",
        );
        return;
    }

    let player_location = {
        let entities = game_state.active_entities.read().await;
        match entities.get(&player.entity_id) {
            Some(e) => e.location.clone(),
            None => return,
        }
    };

    let candidate = find_talk_candidate(game_state, player, &player_location).await;

    match candidate {
        None => {
            messaging::message(
                &game_state.message_tx,
                player.id,
                "There's nobody to talk to here.",
            );
        }
        Some(TalkCandidate::Agent {
            npc_entity_id,
            instructions,
        }) => {
            AgentConversationStarter::new(
                game_state,
                player,
                npc_entity_id,
                instructions,
                initial_message,
            )
            .start()
            .await;
        }
        Some(TalkCandidate::StandardDialog {
            npc_entity_id,
            dialog_root,
        }) => {
            start_standard_dialog(game_state, player, npc_entity_id, dialog_root).await;
        }
    }
}

async fn find_talk_candidate(
    game_state: &Arc<GameState>,
    player: &Player,
    player_location: &crate::game::Location,
) -> Option<TalkCandidate> {
    let entities = game_state.active_entities.read().await;
    entities
        .values()
        .filter(|e| e.id != player.entity_id && &e.location == player_location)
        .find_map(|e| {
            let config_id = e.config_id.as_deref()?;
            let config = game_state.entity_configs.get(config_id)?;
            match &config.persona {
                Some(PersonaConfig::Agent { parsed_persona, .. }) => {
                    let instructions = match parsed_persona {
                        Some(persona) => {
                            let ctx = PersonaContext {
                                trust: 0.0,
                                attributes: HashMap::new(),
                            };
                            persona.to_instructions(&ctx)
                        }
                        None => "You are an NPC in a text adventure game.".to_string(),
                    };
                    Some(TalkCandidate::Agent {
                        npc_entity_id: e.id,
                        instructions,
                    })
                }
                Some(PersonaConfig::Standard {
                    dialog_tree: Some(tree),
                    ..
                }) => Some(TalkCandidate::StandardDialog {
                    npc_entity_id: e.id,
                    dialog_root: tree.clone(),
                }),
                _ => None,
            }
        })
}

async fn start_standard_dialog(
    game_state: &Arc<GameState>,
    player: &Player,
    npc_entity_id: i64,
    dialog_root: DialogLine,
) {
    let engagement_id = game_state
        .engagements
        .add_conversation(player.entity_id, npc_entity_id)
        .await;

    {
        let mut entities = game_state.active_entities.write().await;
        if let Some(npc) = entities.get_mut(&npc_entity_id) {
            let mut state = SimpleConversationState::default();
            state.contexts.insert(
                engagement_id,
                ConversationContext {
                    current_dialog: Some(dialog_root.clone()),
                },
            );
            npc.ai = Some(EntityAI {
                simple_conversation_state: Some(state),
                agent_conversation_state: None,
            });
        }
    }

    let options: Vec<String> = dialog_root
        .responses
        .iter()
        .map(|r| r.text.clone())
        .collect();
    let greeting = pick_text(&dialog_root);
    let msg = format_dialog_message(greeting, &dialog_root.responses);
    messaging::stream_message(game_state.message_tx.clone(), player.id, msg);
    messaging::conversation_started(
        &game_state.message_tx,
        player.id,
        ConversationKind::Dialog,
        options,
    );
}

pub fn pick_text(dialog: &DialogLine) -> &str {
    if dialog.alts.is_empty() {
        &dialog.text
    } else {
        let idx = fastrand::usize(..=dialog.alts.len());
        if idx == 0 {
            &dialog.text
        } else {
            &dialog.alts[idx - 1]
        }
    }
}

pub fn format_dialog_message(text: &str, responses: &[PlayerResponse]) -> String {
    if responses.is_empty() {
        return text.to_string();
    }
    let mut msg = text.to_string();
    for (i, r) in responses.iter().enumerate() {
        msg.push('\n');
        msg.push_str(&format!("[{}] {}", i + 1, r.text));
    }
    msg
}
