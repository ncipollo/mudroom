use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum Message {
    Complete(String),
    Streaming,
}

#[derive(Debug, Clone)]
pub struct PlayerMessage {
    pub player_id: i64,
    pub message: Message,
}

pub fn announce(tx: &broadcast::Sender<PlayerMessage>, player_id: i64, content: impl Into<String>) {
    let _ = tx.send(PlayerMessage {
        player_id,
        message: Message::Complete(content.into()),
    });
}
