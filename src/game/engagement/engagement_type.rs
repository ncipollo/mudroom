use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EngagementType {
    Conversation,
    Battle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_conversation() {
        let json = serde_json::to_string(&EngagementType::Conversation).unwrap();
        assert_eq!(json, "\"conversation\"");
        let restored: EngagementType = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, EngagementType::Conversation);
    }

    #[test]
    fn serde_battle() {
        let json = serde_json::to_string(&EngagementType::Battle).unwrap();
        assert_eq!(json, "\"battle\"");
        let restored: EngagementType = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, EngagementType::Battle);
    }
}
