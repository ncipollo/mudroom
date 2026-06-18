use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattlePhase {
    InnateEffects,
    AttackerPlanning,
    DefenderResponse,
    Resolution,
    Concluded,
}

impl fmt::Display for BattlePhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BattlePhase::InnateEffects => "Innate Effects",
            BattlePhase::AttackerPlanning => "Attacker Planning",
            BattlePhase::DefenderResponse => "Defender Response",
            BattlePhase::Resolution => "Resolution",
            BattlePhase::Concluded => "Concluded",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_formats_human_readable() {
        assert_eq!(BattlePhase::InnateEffects.to_string(), "Innate Effects");
        assert_eq!(
            BattlePhase::AttackerPlanning.to_string(),
            "Attacker Planning"
        );
        assert_eq!(
            BattlePhase::DefenderResponse.to_string(),
            "Defender Response"
        );
        assert_eq!(BattlePhase::Resolution.to_string(), "Resolution");
        assert_eq!(BattlePhase::Concluded.to_string(), "Concluded");
    }

    #[test]
    fn serde_round_trip() {
        let phases = [
            BattlePhase::InnateEffects,
            BattlePhase::AttackerPlanning,
            BattlePhase::DefenderResponse,
            BattlePhase::Resolution,
            BattlePhase::Concluded,
        ];
        for phase in &phases {
            let json = serde_json::to_string(phase).unwrap();
            let restored: BattlePhase = serde_json::from_str(&json).unwrap();
            assert_eq!(&restored, phase);
        }
    }
}
