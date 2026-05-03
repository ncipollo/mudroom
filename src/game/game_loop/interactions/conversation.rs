use std::collections::HashMap;
use std::sync::Arc;

use crate::agent;
use crate::agent::tools::InspectEntity;
use crate::game::config::{DialogLine, PersonaConfig, PersonaContext, PlayerResponse};
use crate::game::entity_ai::{
    AgentConversationContext, AgentConversationState, ConversationContext, EntityAI,
    SimpleConversationState,
};
use crate::game::messaging::{Message, PlayerMessage};
use crate::game::player::Player;
use crate::game::{GameState, messaging};

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

pub async fn process(game_state: &Arc<GameState>, player: &Player) {
    if game_state
        .engagements
        .is_entity_in_conversation(player.entity_id)
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
            start_agent_conversation(game_state, player, npc_entity_id, instructions).await;
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

async fn start_agent_conversation(
    game_state: &Arc<GameState>,
    player: &Player,
    npc_entity_id: i64,
    instructions: String,
) {
    let engagement_id = game_state
        .engagements
        .add_conversation(player.entity_id, npc_entity_id)
        .await;

    {
        let mut entities = game_state.active_entities.write().await;
        if let Some(npc) = entities.get_mut(&npc_entity_id) {
            let mut state = AgentConversationState::default();
            state.contexts.insert(
                engagement_id,
                AgentConversationContext {
                    instructions: instructions.clone(),
                    history: Vec::new(),
                },
            );
            let ai = npc.ai.get_or_insert_with(EntityAI::default);
            ai.agent_conversation_state = Some(state);
        }
    }

    let provider = agent::build_provider(&game_state.mud_config.agent);
    let tx = game_state.message_tx.clone();
    let player_id = player.id;
    let game_state_clone = game_state.clone();

    tokio::spawn(async move {
        let tools: Vec<Box<dyn rig::tool::ToolDyn>> = vec![Box::new(InspectEntity {
            game_state: game_state_clone,
            npc_entity_id,
        })];
        match provider
            .chat(
                &instructions,
                "Greet the player with a brief greeting.",
                &[],
                tools,
            )
            .await
        {
            Ok(text) => messaging::stream_message(tx, player_id, text),
            Err(e) => {
                let _ = tx.send(PlayerMessage {
                    player_id,
                    message: Message::Complete(format!("The NPC seems distracted. ({e})")),
                });
            }
        }
    });
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

    let greeting = pick_text(&dialog_root);
    let msg = format_dialog_message(greeting, &dialog_root.responses);
    messaging::stream_message(game_state.message_tx.clone(), player.id, msg);
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
