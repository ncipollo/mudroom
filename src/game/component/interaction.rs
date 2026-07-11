pub mod direction;
pub mod movement;

pub use direction::Direction;
pub use movement::Movement;

use serde::{Deserialize, Serialize};

use crate::game::engagement::TurnAction;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Interaction {
    Look,
    Help,
    Movement(Movement),
    EngagementAction(TurnAction),
    StartConversation { initial_message: Option<String> },
    EndConversation,
    JoinBattle { engagement_id: i64 },
    LeaveBattle { engagement_id: i64 },
    CheckRoomThreats { room_id: String },
    PlayerDisconnected,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_conversation_serde_no_message() {
        let v = Interaction::StartConversation {
            initial_message: None,
        };
        let json = serde_json::to_string(&v).unwrap();
        println!("none json: {json}");
        let rt: Interaction = serde_json::from_str(&json).unwrap();
        assert_eq!(v, rt);
    }

    #[test]
    fn start_conversation_serde_with_message() {
        let v = Interaction::StartConversation {
            initial_message: Some("hello".into()),
        };
        let json = serde_json::to_string(&v).unwrap();
        println!("some json: {json}");
        let rt: Interaction = serde_json::from_str(&json).unwrap();
        assert_eq!(v, rt);
    }
}
