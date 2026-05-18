pub mod stream;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::game::map::universe::room::Room;

pub use stream::stream_message;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConversationKind {
    Agent,
    Dialog,
}

#[derive(Debug, Clone)]
pub enum StreamingState {
    Streaming,
    Complete,
}

#[derive(Debug, Clone)]
pub enum Message {
    Complete(String),
    Streaming {
        chunk: String,
        state: StreamingState,
    },
    ConversationStarted {
        kind: ConversationKind,
        options: Vec<String>,
    },
    ConversationEnded,
}

#[derive(Debug, Clone)]
pub struct PlayerMessage {
    pub player_id: i64,
    pub message: Message,
}

pub fn message(tx: &broadcast::Sender<PlayerMessage>, player_id: i64, content: impl Into<String>) {
    let _ = tx.send(PlayerMessage {
        player_id,
        message: Message::Complete(content.into()),
    });
}

pub fn conversation_started(
    tx: &broadcast::Sender<PlayerMessage>,
    player_id: i64,
    kind: ConversationKind,
    options: Vec<String>,
) {
    let _ = tx.send(PlayerMessage {
        player_id,
        message: Message::ConversationStarted { kind, options },
    });
}

pub fn conversation_ended(tx: &broadcast::Sender<PlayerMessage>, player_id: i64) {
    let _ = tx.send(PlayerMessage {
        player_id,
        message: Message::ConversationEnded,
    });
}

pub fn message_room_description(
    tx: &broadcast::Sender<PlayerMessage>,
    player_id: i64,
    room: &Room,
) {
    let content = room
        .description
        .standard
        .as_deref()
        .unwrap_or("You look around but see nothing remarkable.")
        .to_string();
    message(tx, player_id, content);
}
