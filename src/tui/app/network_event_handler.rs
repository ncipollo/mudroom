use crate::game::engagement::battle::BattleMessage;
use crate::game::messaging::ConversationKind;
use crate::network::NetworkEvent;
use crate::network::event::BattleSnapshot;

use super::{App, AppMessage, BattleState, GameMode};

impl App {
    pub fn handle_network_event(&mut self, event: NetworkEvent) {
        match event {
            NetworkEvent::StartSession { session_id } => self
                .messages
                .push(AppMessage::normal(format!("Session started: {session_id}"))),
            NetworkEvent::EndSession { session_id } => self
                .messages
                .push(AppMessage::normal(format!("Session ended: {session_id}"))),
            NetworkEvent::Ping => {
                if self.debug {
                    self.messages.push(AppMessage::debug("[ping received]"));
                }
            }
            NetworkEvent::Pong => {
                if self.debug {
                    self.messages.push(AppMessage::debug("[pong received]"));
                }
            }
            NetworkEvent::PlayerSelected {
                player_name,
                player_id,
                entity_id,
                ..
            } => {
                self.current_player_id = Some(player_id);
                self.current_entity_id = Some(entity_id);
                self.streaming_message_index = None;
                self.messages
                    .push(AppMessage::normal(format!("Playing as: {player_name}")));
            }
            NetworkEvent::Message { player_id, content } => {
                if Some(player_id) == self.current_player_id {
                    self.messages.push(AppMessage::normal(content));
                }
            }
            NetworkEvent::MessageChunk {
                player_id,
                chunk,
                is_final,
            } => self.handle_message_chunk(player_id, chunk, is_final),
            NetworkEvent::ConversationStarted { kind, options } => {
                self.handle_conversation_started(kind, options);
            }
            NetworkEvent::ConversationEnded => {
                self.mode = GameMode::Game;
                self.conversation.reset();
            }
            NetworkEvent::BattleStarted {
                engagement_id,
                snapshot,
            } => self.handle_battle_started(engagement_id, snapshot),
            NetworkEvent::BattleUpdate {
                engagement_id,
                snapshot,
                messages,
            } => self.handle_battle_update(engagement_id, snapshot, messages),
            NetworkEvent::BattleEnded { engagement_id: _ } => {
                self.battle = None;
                self.mode = GameMode::Game;
            }
        }
    }

    fn handle_message_chunk(&mut self, player_id: i64, chunk: String, is_final: bool) {
        if Some(player_id) != self.current_player_id {
            return;
        }
        match self.streaming_message_index {
            None => {
                let idx = self.messages.len();
                self.messages.push(AppMessage::normal(chunk));
                if !is_final {
                    self.streaming_message_index = Some(idx);
                }
            }
            Some(idx) => {
                if let Some(msg) = self.messages.get_mut(idx) {
                    msg.text.push_str(&chunk);
                }
                if is_final {
                    self.streaming_message_index = None;
                }
            }
        }
        if is_final {
            self.agent_responding = false;
        }
    }

    fn handle_conversation_started(&mut self, kind: ConversationKind, options: Vec<String>) {
        match kind {
            ConversationKind::Dialog => {
                self.mode = GameMode::StandardConversation;
                self.conversation.options = options;
                self.conversation.selected_index = 0;
            }
            ConversationKind::Agent => {
                self.mode = GameMode::AgentConversation;
                self.messages.clear();
                self.scroll_offset = 0;
            }
        }
    }

    fn handle_battle_started(&mut self, engagement_id: i64, snapshot: BattleSnapshot) {
        self.battle = Some(BattleState::new(engagement_id, snapshot));
        self.mode = GameMode::Battle;
    }

    fn handle_battle_update(
        &mut self,
        engagement_id: i64,
        snapshot: BattleSnapshot,
        messages: Vec<BattleMessage>,
    ) {
        let Some(battle) = &mut self.battle else {
            return;
        };
        if battle.engagement_id != engagement_id {
            return;
        }
        if battle.snapshot.phase != snapshot.phase {
            battle.selected_ability_index = 0;
            battle.queued_ability = None;
        }
        battle.snapshot.participants = snapshot.participants;
        battle.snapshot.phase = snapshot.phase;
        battle.snapshot.countdown_secs = snapshot.countdown_secs;
        battle.snapshot.max_turn_secs = snapshot.max_turn_secs;
        battle.snapshot.available_abilities = snapshot.available_abilities;
        battle.message_log.extend(messages);
    }
}
