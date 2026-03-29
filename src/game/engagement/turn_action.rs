use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TurnAction {
    ApplyEffect { effect_name: String },
    SelectDialogChoice { choice: String },
    SendMessage { content: String },
    Respond { content: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_apply_effect() {
        let action = TurnAction::ApplyEffect {
            effect_name: "fireball".to_string(),
        };
        let json = serde_json::to_string(&action).unwrap();
        let restored: TurnAction = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, action);
    }

    #[test]
    fn serde_send_message() {
        let action = TurnAction::SendMessage {
            content: "hello".to_string(),
        };
        let json = serde_json::to_string(&action).unwrap();
        let restored: TurnAction = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, action);
    }
}
