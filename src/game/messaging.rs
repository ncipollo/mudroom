pub mod stream;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::game::component::Ability;
use crate::game::component::attribute_definition::AttributeType;
use crate::game::engagement::battle::{BattleMessage, BattlePhase};
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
pub struct BattleParticipantInfo {
    pub id: i64,
    pub name: String,
    pub hp_current: i64,
    pub hp_max: i64,
}

#[derive(Debug, Clone)]
pub struct BattleUpdateMessage {
    pub engagement_id: i64,
    pub factions: Vec<String>,
    pub participants: HashMap<String, Vec<BattleParticipantInfo>>,
    pub phase: BattlePhase,
    pub messages: Vec<BattleMessage>,
    pub countdown_secs: u64,
    pub max_turn_secs: u64,
    pub available_abilities: Vec<Ability>,
}

#[derive(Debug, Clone)]
pub struct BattleStartedMessage {
    pub engagement_id: i64,
    pub factions: Vec<String>,
    pub participants: HashMap<String, Vec<BattleParticipantInfo>>,
    pub phase: BattlePhase,
    pub turn_order: Vec<i64>,
    pub countdown_secs: u64,
    pub max_turn_secs: u64,
    pub available_abilities: Vec<Ability>,
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
    BattleStarted(Box<BattleStartedMessage>),
    BattleUpdate(Box<BattleUpdateMessage>),
    BattleEnded {
        engagement_id: i64,
    },
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

pub fn battle_started(
    tx: &broadcast::Sender<PlayerMessage>,
    player_id: i64,
    data: BattleStartedMessage,
) {
    let _ = tx.send(PlayerMessage {
        player_id,
        message: Message::BattleStarted(Box::new(data)),
    });
}

pub fn battle_update(
    tx: &broadcast::Sender<PlayerMessage>,
    player_id: i64,
    data: BattleUpdateMessage,
) {
    let _ = tx.send(PlayerMessage {
        player_id,
        message: Message::BattleUpdate(Box::new(data)),
    });
}

pub fn battle_ended(tx: &broadcast::Sender<PlayerMessage>, player_id: i64, engagement_id: i64) {
    let _ = tx.send(PlayerMessage {
        player_id,
        message: Message::BattleEnded { engagement_id },
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
        .text
        .as_deref()
        .unwrap_or("You look around but see nothing remarkable.")
        .to_string();
    message(tx, player_id, content);
}

pub fn hp_attribute_id(attribute_config: &crate::game::config::AttributeConfig) -> String {
    attribute_config
        .attributes
        .iter()
        .find(|def| matches!(def.attribute_type, AttributeType::HP))
        .map(|def| def.id.clone())
        .unwrap_or_else(|| "hp".to_string())
}
