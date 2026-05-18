use std::sync::Arc;

use crate::agent;
use crate::agent::entity_ai::{AgentConversationContext, AgentConversationState, EntityAI};
use crate::agent::tools::InspectEntity;
use crate::game::messaging::{ConversationKind, Message, PlayerMessage};
use crate::game::player::Player;
use crate::game::{GameState, messaging};

pub struct AgentConversationStarter<'a> {
    game_state: &'a Arc<GameState>,
    player: &'a Player,
    npc_entity_id: i64,
    instructions: String,
}

impl<'a> AgentConversationStarter<'a> {
    pub fn new(
        game_state: &'a Arc<GameState>,
        player: &'a Player,
        npc_entity_id: i64,
        instructions: String,
    ) -> Self {
        Self {
            game_state,
            player,
            npc_entity_id,
            instructions,
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
}
