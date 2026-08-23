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
    /// Like `Look`, but marks a genuine room arrival (movement or connect) rather than a
    /// player-issued `look` — used to gate visit-triggered world loot respawns.
    EnterRoom,
    LookAt {
        target: String,
    },
    Help,
    Movement(Movement),
    EngagementAction(TurnAction),
    StartConversation {
        initial_message: Option<String>,
    },
    EndConversation,
    JoinBattle {
        engagement_id: i64,
    },
    LeaveBattle {
        engagement_id: i64,
    },
    CheckRoomThreats {
        room_id: String,
    },
    PlayerDisconnected {
        client_id: String,
        epoch: u64,
    },
    Take {
        target: String,
    },
    OpenInventory,
    UseItem {
        item_id: i64,
    },
    EquipItem {
        item_id: i64,
    },
    UnequipItem {
        item_id: i64,
    },
    DropItem {
        item_id: i64,
    },
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

    #[test]
    fn look_at_serde_round_trip() {
        let v = Interaction::LookAt {
            target: "sword".to_string(),
        };
        let json = serde_json::to_string(&v).unwrap();
        let rt: Interaction = serde_json::from_str(&json).unwrap();
        assert_eq!(v, rt);
    }

    #[test]
    fn take_serde_round_trip() {
        let v = Interaction::Take {
            target: "sword".to_string(),
        };
        let json = serde_json::to_string(&v).unwrap();
        let rt: Interaction = serde_json::from_str(&json).unwrap();
        assert_eq!(v, rt);
    }

    #[test]
    fn use_item_serde_round_trip() {
        let v = Interaction::UseItem { item_id: 7 };
        let json = serde_json::to_string(&v).unwrap();
        let rt: Interaction = serde_json::from_str(&json).unwrap();
        assert_eq!(v, rt);
    }

    #[test]
    fn equip_item_serde_round_trip() {
        let v = Interaction::EquipItem { item_id: 7 };
        let json = serde_json::to_string(&v).unwrap();
        let rt: Interaction = serde_json::from_str(&json).unwrap();
        assert_eq!(v, rt);
    }

    #[test]
    fn unequip_item_serde_round_trip() {
        let v = Interaction::UnequipItem { item_id: 7 };
        let json = serde_json::to_string(&v).unwrap();
        let rt: Interaction = serde_json::from_str(&json).unwrap();
        assert_eq!(v, rt);
    }

    #[test]
    fn drop_item_serde_round_trip() {
        let v = Interaction::DropItem { item_id: 7 };
        let json = serde_json::to_string(&v).unwrap();
        let rt: Interaction = serde_json::from_str(&json).unwrap();
        assert_eq!(v, rt);
    }
}
