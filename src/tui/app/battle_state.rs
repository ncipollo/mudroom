use crate::game::engagement::battle::{BattleMessage, BattlePhase};
use crate::network::event::{BattleSnapshot, ParticipantInfo};

#[derive(Debug, Clone, PartialEq)]
pub enum BattleFocus {
    Abilities,
    EntityList,
}

#[derive(Debug, Clone)]
pub struct TargetDialog {
    pub pending_ability_id: String,
    pub selected_index: usize,
    pub targets: Vec<ParticipantInfo>,
}

#[derive(Debug, Clone)]
pub struct BattleState {
    pub engagement_id: i64,
    pub snapshot: BattleSnapshot,
    pub message_log: Vec<BattleMessage>,
    pub selected_ability_index: usize,
    pub selected_entity_index: usize,
    pub entity_scroll: usize,
    pub focus: BattleFocus,
    pub dialog: Option<TargetDialog>,
}

impl BattleState {
    pub fn new(engagement_id: i64, snapshot: BattleSnapshot) -> Self {
        Self {
            engagement_id,
            snapshot,
            message_log: Vec::new(),
            selected_ability_index: 0,
            selected_entity_index: 0,
            entity_scroll: 0,
            focus: BattleFocus::Abilities,
            dialog: None,
        }
    }

    pub fn select_next_ability(&mut self) {
        let len = self.snapshot.available_abilities.len();
        if len > 0 {
            self.selected_ability_index = (self.selected_ability_index + 1) % len;
        }
    }

    pub fn select_prev_ability(&mut self) {
        let len = self.snapshot.available_abilities.len();
        if len > 0 {
            self.selected_ability_index = (self.selected_ability_index + len - 1) % len;
        }
    }

    pub fn select_next_entity(&mut self) {
        let len: usize = self.snapshot.participants.values().map(|v| v.len()).sum();
        if len > 0 {
            self.selected_entity_index = (self.selected_entity_index + 1) % len;
        }
    }

    pub fn select_prev_entity(&mut self) {
        let len: usize = self.snapshot.participants.values().map(|v| v.len()).sum();
        if len > 0 {
            self.selected_entity_index = (self.selected_entity_index + len - 1) % len;
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            BattleFocus::Abilities => BattleFocus::EntityList,
            BattleFocus::EntityList => BattleFocus::Abilities,
        };
    }

    pub fn open_target_dialog(&mut self, ability_id: String) {
        let targets: Vec<ParticipantInfo> = self
            .snapshot
            .participants
            .values()
            .flat_map(|infos| infos.iter().cloned())
            .collect();
        self.dialog = Some(TargetDialog {
            pending_ability_id: ability_id,
            selected_index: 0,
            targets,
        });
    }

    pub fn close_target_dialog(&mut self) {
        self.dialog = None;
    }

    pub fn target_dialog_next(&mut self) {
        if let Some(dialog) = &mut self.dialog {
            let len = dialog.targets.len();
            if len > 0 {
                dialog.selected_index = (dialog.selected_index + 1) % len;
            }
        }
    }

    pub fn target_dialog_prev(&mut self) {
        if let Some(dialog) = &mut self.dialog {
            let len = dialog.targets.len();
            if len > 0 {
                dialog.selected_index = (dialog.selected_index + len - 1) % len;
            }
        }
    }

    pub fn dialog_target_id(&self) -> Option<i64> {
        let dialog = self.dialog.as_ref()?;
        dialog.targets.get(dialog.selected_index).map(|p| p.id)
    }

    pub fn is_player_turn(&self) -> bool {
        self.snapshot.phase == BattlePhase::AttackerPlanning
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::game::component::Ability;
    use crate::network::event::BattleSnapshot;

    fn make_snapshot(phase: BattlePhase) -> BattleSnapshot {
        BattleSnapshot {
            factions: vec!["player".to_string(), "enemy".to_string()],
            participants: HashMap::new(),
            phase,
            turn_order: vec![],
            countdown_ticks: 0,
            max_turn_ticks: 30,
            available_abilities: vec![],
        }
    }

    fn make_state(phase: BattlePhase) -> BattleState {
        BattleState::new(1, make_snapshot(phase))
    }

    #[test]
    fn is_player_turn_true_during_attacker_planning() {
        assert!(make_state(BattlePhase::AttackerPlanning).is_player_turn());
    }

    #[test]
    fn is_player_turn_false_during_defender_response() {
        assert!(!make_state(BattlePhase::DefenderResponse).is_player_turn());
    }

    #[test]
    fn is_player_turn_false_during_innate_effects() {
        assert!(!make_state(BattlePhase::InnateEffects).is_player_turn());
    }

    #[test]
    fn is_player_turn_false_during_resolution() {
        assert!(!make_state(BattlePhase::Resolution).is_player_turn());
    }

    #[test]
    fn is_player_turn_false_during_concluded() {
        assert!(!make_state(BattlePhase::Concluded).is_player_turn());
    }
}
