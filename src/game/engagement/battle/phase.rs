use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BattlePhase {
    InnateEffects,
    Planning { faction: String },
    Response { faction: String },
    Resolution,
    Concluded,
}

impl fmt::Display for BattlePhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BattlePhase::InnateEffects => "Innate Effects".to_string(),
            BattlePhase::Planning { faction } => format!("{faction} Planning"),
            BattlePhase::Response { faction } => format!("Response to {faction}"),
            BattlePhase::Resolution => "Resolution".to_string(),
            BattlePhase::Concluded => "Concluded".to_string(),
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
            BattlePhase::Planning {
                faction: "player".into()
            }
            .to_string(),
            "player Planning"
        );
        assert_eq!(
            BattlePhase::Response {
                faction: "enemy".into()
            }
            .to_string(),
            "Response to enemy"
        );
        assert_eq!(BattlePhase::Resolution.to_string(), "Resolution");
        assert_eq!(BattlePhase::Concluded.to_string(), "Concluded");
    }

    #[test]
    fn serde_round_trip() {
        let phases = [
            BattlePhase::InnateEffects,
            BattlePhase::Planning {
                faction: "player".into(),
            },
            BattlePhase::Response {
                faction: "enemy".into(),
            },
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
