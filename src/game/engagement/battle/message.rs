use std::fmt;

use serde::{Deserialize, Serialize};

use crate::game::engagement::battle::phase::BattlePhase;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BattleMessage {
    PhaseChange {
        phase: BattlePhase,
    },
    AbilityCast {
        caster_name: String,
        target_name: String,
        ability_name: String,
    },
    EntityDied {
        name: String,
    },
    PendingAttack {
        caster_name: String,
        ability_name: String,
        target_name: String,
        target_id: i64,
    },
    Meta(String),
}

impl fmt::Display for BattleMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BattleMessage::PhaseChange { phase } => write!(f, "[Battle: {phase}]"),
            BattleMessage::AbilityCast {
                caster_name,
                target_name,
                ability_name,
            } => write!(f, "{caster_name} uses {ability_name} on {target_name}."),
            BattleMessage::EntityDied { name } => write!(f, "{name} has been defeated!"),
            BattleMessage::PendingAttack {
                caster_name,
                ability_name,
                target_name,
                ..
            } => write!(
                f,
                "{caster_name} is preparing {ability_name} → {target_name}"
            ),
            BattleMessage::Meta(msg) => write!(f, "{msg}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_change_display() {
        let msg = BattleMessage::PhaseChange {
            phase: BattlePhase::Resolution,
        };
        assert_eq!(msg.to_string(), "[Battle: Resolution]");
    }

    #[test]
    fn ability_cast_display() {
        let msg = BattleMessage::AbilityCast {
            caster_name: "Goblin".to_string(),
            target_name: "Player".to_string(),
            ability_name: "Slash".to_string(),
        };
        assert_eq!(msg.to_string(), "Goblin uses Slash on Player.");
    }

    #[test]
    fn entity_died_display() {
        let msg = BattleMessage::EntityDied {
            name: "Goblin".to_string(),
        };
        assert_eq!(msg.to_string(), "Goblin has been defeated!");
    }

    #[test]
    fn meta_display() {
        let msg = BattleMessage::Meta("The battle has ended.".to_string());
        assert_eq!(msg.to_string(), "The battle has ended.");
    }

    #[test]
    fn pending_attack_display() {
        let msg = BattleMessage::PendingAttack {
            caster_name: "Goblin".to_string(),
            ability_name: "Slash".to_string(),
            target_name: "Player".to_string(),
            target_id: 1,
        };
        assert_eq!(msg.to_string(), "Goblin is preparing Slash → Player");
    }
}
