use std::sync::Arc;

use crate::game::config::{DialogLine, PersonaConfig, PlayerResponse};
use crate::game::entity_ai::{ConversationContext, EntityAI, SimpleConversationState};
use crate::game::player::Player;
use crate::game::{GameState, messaging};

enum TalkCandidate {
    AgentStub {
        label: String,
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

    // Find a talkable entity in the same room
    let candidate = {
        let entities = game_state.active_entities.read().await;
        entities
            .values()
            .filter(|e| e.id != player.entity_id && e.location == player_location)
            .find_map(|e| {
                let config_id = e.config_id.as_deref()?;
                let config = game_state.entity_configs.get(config_id)?;
                match &config.persona {
                    Some(PersonaConfig::Agent { .. }) => {
                        let label = e.description.as_deref().unwrap_or("entity").to_string();
                        Some(TalkCandidate::AgentStub { label })
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
    };

    match candidate {
        None => {
            messaging::message(
                &game_state.message_tx,
                player.id,
                "There's nobody to talk to here.",
            );
        }
        Some(TalkCandidate::AgentStub { label }) => {
            messaging::message(
                &game_state.message_tx,
                player.id,
                format!("The {label} doesn't seem ready to talk."),
            );
        }
        Some(TalkCandidate::StandardDialog {
            npc_entity_id,
            dialog_root,
        }) => {
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
                    });
                }
            }

            let greeting = pick_text(&dialog_root);
            let msg = format_dialog_message(greeting, &dialog_root.responses);
            messaging::message(&game_state.message_tx, player.id, msg);
        }
    }
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
