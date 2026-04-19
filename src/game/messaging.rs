use tokio::sync::broadcast;

use crate::game::map::universe::room::Room;

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
}

#[derive(Debug, Clone)]
pub struct PlayerMessage {
    pub player_id: i64,
    pub message: Message,
}

pub fn stream_message(
    tx: broadcast::Sender<PlayerMessage>,
    player_id: i64,
    content: impl Into<String>,
) {
    let content = content.into();
    tokio::spawn(async move {
        let words: Vec<&str> = content.split(' ').filter(|s| !s.is_empty()).collect();
        let total = words.len();
        for (i, word) in words.iter().enumerate() {
            let is_last = i + 1 == total;
            let chunk = if is_last {
                word.to_string()
            } else {
                format!("{word} ")
            };
            let state = if is_last {
                StreamingState::Complete
            } else {
                StreamingState::Streaming
            };
            let _ = tx.send(PlayerMessage {
                player_id,
                message: Message::Streaming { chunk, state },
            });
            if !is_last {
                tokio::time::sleep(tokio::time::Duration::from_millis(40)).await;
            }
        }
    });
}

pub fn message(tx: &broadcast::Sender<PlayerMessage>, player_id: i64, content: impl Into<String>) {
    let _ = tx.send(PlayerMessage {
        player_id,
        message: Message::Complete(content.into()),
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
