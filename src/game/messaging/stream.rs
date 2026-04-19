use tokio::sync::broadcast;

use super::{Message, PlayerMessage, StreamingState};

const STREAM_DELAY_MS: u64 = 40;

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
                tokio::time::sleep(tokio::time::Duration::from_millis(STREAM_DELAY_MS)).await;
            }
        }
    });
}
