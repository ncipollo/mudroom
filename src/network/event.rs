use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::game::component::Ability;
use crate::game::engagement::battle::{BattleMessage, BattlePhase};
use crate::game::messaging::ConversationKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfoResponse {
    pub server_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartResponse {
    pub client_id: String,
    pub server_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerListResponse {
    pub players: Vec<PlayerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassListResponse {
    pub classes: Vec<ClassInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeStyleInfo {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeInfo {
    pub id: String,
    pub styles: HashMap<String, ThemeStyleInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeListResponse {
    pub themes: Vec<ThemeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticipantInfo {
    pub id: i64,
    pub name: String,
    pub hp_current: i64,
    pub hp_max: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InventoryItemInfo {
    pub item_id: i64,
    pub name: String,
    pub item_type: String,
    pub description: String,
    pub usable: bool,
    pub equippable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InventorySlotInfo {
    pub slot_name: String,
    pub equipped: Option<InventoryItemInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BattleSnapshot {
    pub factions: Vec<String>,
    pub participants: HashMap<String, Vec<ParticipantInfo>>,
    pub phase: BattlePhase,
    pub turn_order: Vec<i64>,
    pub countdown_secs: u64,
    pub max_turn_secs: u64,
    pub available_abilities: Vec<Ability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NetworkEvent {
    StartSession {
        session_id: String,
    },
    EndSession {
        session_id: String,
    },
    Ping,
    Pong,
    PlayerSelected {
        client_id: String,
        player_id: i64,
        player_name: String,
        entity_id: i64,
    },
    Message {
        player_id: i64,
        content: String,
        theme: Option<String>,
    },
    MessageChunk {
        player_id: i64,
        chunk: String,
        is_final: bool,
    },
    ConversationStarted {
        kind: ConversationKind,
        options: Vec<String>,
    },
    ConversationEnded,
    BattleStarted {
        engagement_id: i64,
        snapshot: BattleSnapshot,
    },
    BattleUpdate {
        engagement_id: i64,
        snapshot: BattleSnapshot,
        messages: Vec<BattleMessage>,
    },
    BattleEnded {
        engagement_id: i64,
    },
    InventoryOpened {
        slots: Vec<InventorySlotInfo>,
        bag: Vec<InventoryItemInfo>,
        bag_size: usize,
    },
    PlayerStatsUpdated {
        hp_current: i64,
        hp_max: i64,
        mp_current: i64,
        mp_max: i64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_ping() {
        let json = serde_json::to_string(&NetworkEvent::Ping).unwrap();
        assert_eq!(json, r#"{"type":"ping"}"#);
    }

    #[test]
    fn serialize_pong() {
        let json = serde_json::to_string(&NetworkEvent::Pong).unwrap();
        assert_eq!(json, r#"{"type":"pong"}"#);
    }

    #[test]
    fn serialize_start_session() {
        let json = serde_json::to_string(&NetworkEvent::StartSession {
            session_id: "abc".to_string(),
        })
        .unwrap();
        assert_eq!(json, r#"{"type":"start_session","session_id":"abc"}"#);
    }

    #[test]
    fn serialize_end_session() {
        let json = serde_json::to_string(&NetworkEvent::EndSession {
            session_id: "abc".to_string(),
        })
        .unwrap();
        assert_eq!(json, r#"{"type":"end_session","session_id":"abc"}"#);
    }

    #[test]
    fn round_trip_ping() {
        let event = NetworkEvent::Ping;
        let json = serde_json::to_string(&event).unwrap();
        let decoded: NetworkEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, decoded);
    }

    #[test]
    fn round_trip_message_with_theme() {
        let event = NetworkEvent::Message {
            player_id: 1,
            content: "A cold wind blows.".to_string(),
            theme: Some("eerie".to_string()),
        };
        let json = serde_json::to_string(&event).unwrap();
        let decoded: NetworkEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, decoded);
    }

    #[test]
    fn serialize_player_stats_updated() {
        let json = serde_json::to_string(&NetworkEvent::PlayerStatsUpdated {
            hp_current: 40,
            hp_max: 100,
            mp_current: 50,
            mp_max: 50,
        })
        .unwrap();
        assert_eq!(
            json,
            r#"{"type":"player_stats_updated","hp_current":40,"hp_max":100,"mp_current":50,"mp_max":50}"#
        );
    }

    #[test]
    fn round_trip_player_stats_updated() {
        let event = NetworkEvent::PlayerStatsUpdated {
            hp_current: 40,
            hp_max: 100,
            mp_current: 50,
            mp_max: 50,
        };
        let json = serde_json::to_string(&event).unwrap();
        let decoded: NetworkEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, decoded);
    }

    #[test]
    fn round_trip_inventory_opened() {
        let event = NetworkEvent::InventoryOpened {
            slots: vec![
                InventorySlotInfo {
                    slot_name: "weapon".to_string(),
                    equipped: Some(InventoryItemInfo {
                        item_id: 1,
                        name: "Sword".to_string(),
                        item_type: "weapon".to_string(),
                        description: "A sharp blade.".to_string(),
                        usable: false,
                        equippable: true,
                    }),
                },
                InventorySlotInfo {
                    slot_name: "armor".to_string(),
                    equipped: None,
                },
            ],
            bag: vec![InventoryItemInfo {
                item_id: 2,
                name: "Potion".to_string(),
                item_type: "consumable".to_string(),
                description: "Restores health.".to_string(),
                usable: true,
                equippable: false,
            }],
            bag_size: 20,
        };
        let json = serde_json::to_string(&event).unwrap();
        let decoded: NetworkEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, decoded);
    }

    #[test]
    fn round_trip_theme_info() {
        let mut styles = HashMap::new();
        styles.insert(
            "bold".to_string(),
            ThemeStyleInfo {
                fg: Some("red".to_string()),
                bg: None,
                modifiers: vec!["bold".to_string()],
            },
        );
        let theme = ThemeInfo {
            id: "eerie".to_string(),
            styles,
        };
        let json = serde_json::to_string(&theme).unwrap();
        let decoded: ThemeInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(theme, decoded);
    }

    #[test]
    fn round_trip_start_session() {
        let event = NetworkEvent::StartSession {
            session_id: "xyz".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let decoded: NetworkEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, decoded);
    }
}
