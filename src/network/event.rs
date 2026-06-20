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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticipantInfo {
    pub id: i64,
    pub name: String,
    pub hp_current: i64,
    pub hp_max: i64,
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
    },
    Message {
        player_id: i64,
        content: String,
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
    fn round_trip_start_session() {
        let event = NetworkEvent::StartSession {
            session_id: "xyz".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let decoded: NetworkEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, decoded);
    }
}
