use crate::network::NetworkEvent;

use super::{App, AppMessage, GameMode};

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
                ..
            } => {
                self.current_player_id = Some(player_id);
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
            } => {
                if Some(player_id) == self.current_player_id {
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
                }
            }
            NetworkEvent::ConversationStarted { options } => {
                self.mode = GameMode::StandardConversation;
                self.conversation.options = options;
                self.conversation.selected_index = 0;
            }
            NetworkEvent::ConversationEnded => {
                self.mode = GameMode::Game;
                self.conversation.reset();
            }
        }
    }
}
