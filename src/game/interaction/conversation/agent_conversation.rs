use std::sync::Arc;

use crate::agent;
use crate::agent::entity_ai::{
    AgentConversationContext, AgentConversationState, AgentMessage, AgentRole, EntityAI,
};
use crate::agent::tools::InspectEntity;
use crate::game::messaging::{ConversationKind, Message, PlayerMessage};
use crate::game::player::Player;
use crate::game::{GameState, messaging};

pub struct AgentConversationStarter<'a> {
    game_state: &'a Arc<GameState>,
    player: &'a Player,
    npc_entity_id: i64,
    instructions: String,
    initial_message: Option<String>,
}

impl<'a> AgentConversationStarter<'a> {
    pub fn new(
        game_state: &'a Arc<GameState>,
        player: &'a Player,
        npc_entity_id: i64,
        instructions: String,
        initial_message: Option<String>,
    ) -> Self {
        Self {
            game_state,
            player,
            npc_entity_id,
            instructions,
            initial_message,
        }
    }

    pub async fn start(&self) {
        let engagement_id = self
            .game_state
            .engagements
            .add_conversation(self.player.entity_id, self.npc_entity_id)
            .await;

        {
            let mut entities = self.game_state.active_entities.write().await;
            if let Some(npc) = entities.get_mut(&self.npc_entity_id) {
                let mut state = AgentConversationState::default();
                state.contexts.insert(
                    engagement_id,
                    AgentConversationContext {
                        instructions: self.instructions.clone(),
                        history: Vec::new(),
                    },
                );
                let ai = npc.ai.get_or_insert_with(EntityAI::default);
                ai.agent_conversation_state = Some(state);
            }
        }

        messaging::conversation_started(
            &self.game_state.message_tx,
            self.player.id,
            ConversationKind::Agent,
            vec![],
        );

        let provider = agent::build_provider(&self.game_state.mud_config.agent);
        let tx = self.game_state.message_tx.clone();
        let player_id = self.player.id;
        let game_state_clone = self.game_state.clone();
        let instructions = self.instructions.clone();
        let npc_entity_id = self.npc_entity_id;
        let initial_message = self.initial_message.clone();

        tokio::spawn(async move {
            let tools: Vec<Box<dyn rig::tool::ToolDyn>> = vec![Box::new(InspectEntity {
                game_state: game_state_clone.clone(),
                npc_entity_id,
            })];

            let prompt = initial_message
                .as_deref()
                .unwrap_or("Greet the player with a brief greeting.");

            match provider.chat(&instructions, prompt, &[], tools).await {
                Ok(response) => {
                    if let Some(msg) = &initial_message {
                        append_exchange(
                            &game_state_clone,
                            npc_entity_id,
                            engagement_id,
                            msg,
                            &response,
                        )
                        .await;
                    }
                    messaging::stream_message(tx, player_id, response);
                }
                Err(e) => {
                    let _ = tx.send(PlayerMessage {
                        player_id,
                        message: Message::Complete(format!("The NPC seems distracted. ({e})")),
                    });
                }
            }
        });
    }
}

async fn append_exchange(
    game_state: &Arc<GameState>,
    npc_entity_id: i64,
    engagement_id: i64,
    player_message: &str,
    agent_response: &str,
) {
    let mut entities = game_state.active_entities.write().await;
    if let Some(npc) = entities.get_mut(&npc_entity_id)
        && let Some(ai) = npc.ai.as_mut()
        && let Some(state) = ai.agent_conversation_state.as_mut()
        && let Some(ctx) = state.contexts.get_mut(&engagement_id)
    {
        ctx.history.push(AgentMessage {
            role: AgentRole::Player,
            content: player_message.to_string(),
        });
        ctx.history.push(AgentMessage {
            role: AgentRole::Agent,
            content: agent_response.to_string(),
        });
    }
}
