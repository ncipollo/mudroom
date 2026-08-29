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
pub struct InventoryItemInfo {
    pub item_id: i64,
    pub name: String,
    pub item_type: String,
    pub description: String,
    pub usable: bool,
    pub equippable: bool,
}

#[derive(Debug, Clone)]
pub struct InventorySlotInfo {
    pub slot_name: String,
    pub item_types: Vec<String>,
    pub equipped: Option<InventoryItemInfo>,
}

#[derive(Debug, Clone)]
pub struct InventoryOpenedMessage {
    pub slots: Vec<InventorySlotInfo>,
    pub bag: Vec<InventoryItemInfo>,
    pub bag_size: usize,
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
    Complete {
        content: String,
        theme: Option<String>,
    },
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
    InventoryOpened(Box<InventoryOpenedMessage>),
    PlayerStatsUpdated {
        hp_current: i64,
        hp_max: i64,
        mp_current: i64,
        mp_max: i64,
    },
}

#[derive(Debug, Clone)]
pub struct PlayerMessage {
    pub player_id: i64,
    pub message: Message,
}

pub fn message(tx: &broadcast::Sender<PlayerMessage>, player_id: i64, content: impl Into<String>) {
    message_themed(tx, player_id, content, None);
}

pub fn message_themed(
    tx: &broadcast::Sender<PlayerMessage>,
    player_id: i64,
    content: impl Into<String>,
    theme: Option<String>,
) {
    let _ = tx.send(PlayerMessage {
        player_id,
        message: Message::Complete {
            content: content.into(),
            theme,
        },
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

pub fn player_stats_updated(
    tx: &broadcast::Sender<PlayerMessage>,
    player_id: i64,
    hp_current: i64,
    hp_max: i64,
    mp_current: i64,
    mp_max: i64,
) {
    let _ = tx.send(PlayerMessage {
        player_id,
        message: Message::PlayerStatsUpdated {
            hp_current,
            hp_max,
            mp_current,
            mp_max,
        },
    });
}

pub fn inventory_opened(
    tx: &broadcast::Sender<PlayerMessage>,
    player_id: i64,
    data: InventoryOpenedMessage,
) {
    let _ = tx.send(PlayerMessage {
        player_id,
        message: Message::InventoryOpened(Box::new(data)),
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
    theme: Option<String>,
) {
    let content = room
        .description
        .text
        .as_deref()
        .unwrap_or("You look around but see nothing remarkable.")
        .to_string();
    message_themed(tx, player_id, content, theme);
}

pub fn hp_attribute_id(attribute_config: &crate::game::config::AttributeConfig) -> String {
    attribute_id_for(attribute_config, |t| matches!(t, AttributeType::HP), "hp")
}

pub fn mp_attribute_id(attribute_config: &crate::game::config::AttributeConfig) -> String {
    attribute_id_for(attribute_config, |t| matches!(t, AttributeType::MP), "mp")
}

fn attribute_id_for(
    attribute_config: &crate::game::config::AttributeConfig,
    matches_type: impl Fn(&AttributeType) -> bool,
    fallback: &str,
) -> String {
    attribute_config
        .attributes
        .iter()
        .find(|def| matches_type(&def.attribute_type))
        .map(|def| def.id.clone())
        .unwrap_or_else(|| fallback.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::config::AttributeConfig;

    #[test]
    fn hp_attribute_id_reads_default_config() {
        assert_eq!(hp_attribute_id(&AttributeConfig::default_config()), "hp");
    }

    #[test]
    fn mp_attribute_id_reads_default_config() {
        assert_eq!(mp_attribute_id(&AttributeConfig::default_config()), "mp");
    }

    #[test]
    fn hp_attribute_id_falls_back_when_missing() {
        let config = AttributeConfig { attributes: vec![] };
        assert_eq!(hp_attribute_id(&config), "hp");
    }

    #[test]
    fn mp_attribute_id_falls_back_when_missing() {
        let config = AttributeConfig { attributes: vec![] };
        assert_eq!(mp_attribute_id(&config), "mp");
    }

    #[tokio::test]
    async fn player_stats_updated_sends_expected_message() {
        let (tx, mut rx) = broadcast::channel(4);

        player_stats_updated(&tx, 7, 40, 100, 20, 50);

        let msg = rx.try_recv().expect("expected a PlayerMessage");
        assert_eq!(msg.player_id, 7);
        assert!(matches!(
            msg.message,
            Message::PlayerStatsUpdated {
                hp_current: 40,
                hp_max: 100,
                mp_current: 20,
                mp_max: 50,
            }
        ));
    }
}
