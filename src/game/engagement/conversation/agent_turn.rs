use std::sync::Arc;

use crate::agent;
use crate::agent::entity_ai::{AgentMessage, AgentRole};
use crate::agent::tools::InspectEntity;
use crate::game::player::Player;
use crate::game::{GameState, messaging};

pub struct AgentTurnHandler<'a> {
    game_state: &'a Arc<GameState>,
    player: &'a Player,
    npc_entity_id: i64,
    engagement_id: i64,
}

impl<'a> AgentTurnHandler<'a> {
    pub fn new(
        game_state: &'a Arc<GameState>,
        player: &'a Player,
        npc_entity_id: i64,
        engagement_id: i64,
    ) -> Self {
        Self {
            game_state,
            player,
            npc_entity_id,
            engagement_id,
        }
    }

    pub async fn has_agent_state(&self) -> bool {
        let entities = self.game_state.active_entities.read().await;
        entities
            .get(&self.npc_entity_id)
            .and_then(|e| e.ai.as_ref())
            .and_then(|ai| ai.agent_conversation_state.as_ref())
            .map(|s| s.contexts.contains_key(&self.engagement_id))
            .unwrap_or(false)
    }

    pub async fn handle(&self, player_message: &str) {
        let Some((instructions, history)) = self.load_context().await else {
            self.game_state.engagements.remove(self.engagement_id).await;
            return;
        };

        let provider = agent::build_provider(&self.game_state.mud_config.agent);
        let tools: Vec<Box<dyn rig::tool::ToolDyn>> = vec![Box::new(InspectEntity {
            game_state: self.game_state.clone(),
            npc_entity_id: self.npc_entity_id,
        })];

        match provider
            .chat(&instructions, player_message, &history, tools)
            .await
        {
            Ok(response) => {
                self.append_history(player_message, &response).await;
                messaging::stream_message(
                    self.game_state.message_tx.clone(),
                    self.player.id,
                    response,
                );
            }
            Err(e) => {
                messaging::message(
                    &self.game_state.message_tx,
                    self.player.id,
                    format!("The NPC pauses, seeming confused. ({e})"),
                );
            }
        }
    }

    async fn load_context(&self) -> Option<(String, Vec<AgentMessage>)> {
        let entities = self.game_state.active_entities.read().await;
        entities
            .get(&self.npc_entity_id)
            .and_then(|e| e.ai.as_ref())
            .and_then(|ai| ai.agent_conversation_state.as_ref())
            .and_then(|s| s.contexts.get(&self.engagement_id))
            .map(|ctx| (ctx.instructions.clone(), ctx.history.clone()))
    }

    async fn append_history(&self, player_message: &str, response: &str) {
        let mut entities = self.game_state.active_entities.write().await;
        if let Some(npc) = entities.get_mut(&self.npc_entity_id)
            && let Some(ai) = npc.ai.as_mut()
            && let Some(state) = ai.agent_conversation_state.as_mut()
            && let Some(ctx) = state.contexts.get_mut(&self.engagement_id)
        {
            ctx.history.push(AgentMessage {
                role: AgentRole::Player,
                content: player_message.to_string(),
            });
            ctx.history.push(AgentMessage {
                role: AgentRole::Agent,
                content: response.to_string(),
            });
        }
    }
}
