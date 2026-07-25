use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BattlePhase {
    ResetAttributes { faction: String },
    AnnounceState { faction: String },
    ApplyEffects { faction: String },
    DeclareAttacks { faction: String },
    DeclareDefense { faction: String },
    ResolveAbilities,
    ResolveEntityState,
    Cleanup,
    VictoryCheck,
    Concluded,
}

impl fmt::Display for BattlePhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BattlePhase::ResetAttributes { faction } => format!("{faction} Reset Attributes"),
            BattlePhase::AnnounceState { faction } => format!("{faction} Announce State"),
            BattlePhase::ApplyEffects { faction } => format!("{faction} Apply Effects"),
            BattlePhase::DeclareAttacks { faction } => format!("{faction} Declare Attacks"),
            BattlePhase::DeclareDefense { faction } => format!("Declare Defense against {faction}"),
            BattlePhase::ResolveAbilities => "Resolve Abilities".to_string(),
            BattlePhase::ResolveEntityState => "Resolve Entity State".to_string(),
            BattlePhase::Cleanup => "Cleanup".to_string(),
            BattlePhase::VictoryCheck => "Victory Check".to_string(),
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
        assert_eq!(
            BattlePhase::ResetAttributes {
                faction: "player".into()
            }
            .to_string(),
            "player Reset Attributes"
        );
        assert_eq!(
            BattlePhase::AnnounceState {
                faction: "player".into()
            }
            .to_string(),
            "player Announce State"
        );
        assert_eq!(
            BattlePhase::ApplyEffects {
                faction: "player".into()
            }
            .to_string(),
            "player Apply Effects"
        );
        assert_eq!(
            BattlePhase::DeclareAttacks {
                faction: "player".into()
            }
            .to_string(),
            "player Declare Attacks"
        );
        assert_eq!(
            BattlePhase::DeclareDefense {
                faction: "enemy".into()
            }
            .to_string(),
            "Declare Defense against enemy"
        );
        assert_eq!(
            BattlePhase::ResolveAbilities.to_string(),
            "Resolve Abilities"
        );
        assert_eq!(
            BattlePhase::ResolveEntityState.to_string(),
            "Resolve Entity State"
        );
        assert_eq!(BattlePhase::Cleanup.to_string(), "Cleanup");
        assert_eq!(BattlePhase::VictoryCheck.to_string(), "Victory Check");
        assert_eq!(BattlePhase::Concluded.to_string(), "Concluded");
    }

    #[test]
    fn serde_round_trip() {
        let phases = [
            BattlePhase::ResetAttributes {
                faction: "player".into(),
            },
            BattlePhase::AnnounceState {
                faction: "player".into(),
            },
            BattlePhase::ApplyEffects {
                faction: "player".into(),
            },
            BattlePhase::DeclareAttacks {
                faction: "player".into(),
            },
            BattlePhase::DeclareDefense {
                faction: "enemy".into(),
            },
            BattlePhase::ResolveAbilities,
            BattlePhase::ResolveEntityState,
            BattlePhase::Cleanup,
            BattlePhase::VictoryCheck,
            BattlePhase::Concluded,
        ];
        for phase in &phases {
            let json = serde_json::to_string(phase).unwrap();
            let restored: BattlePhase = serde_json::from_str(&json).unwrap();
            assert_eq!(&restored, phase);
        }
    }
}
