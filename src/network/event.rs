use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfoResponse {
    pub server_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartResponse {
    pub client_id: String,
    pub server_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NetworkEvent {
    StartSession { session_id: String },
    EndSession { session_id: String },
    Ping,
    Pong,
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
