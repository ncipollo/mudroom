mod log_entry;
mod reveal;
mod target_dialog;

pub use log_entry::BattleLogEntry;
pub use target_dialog::TargetDialog;

use std::collections::VecDeque;

use crate::game::component::{Ability, AbilityRole, Description};
use crate::game::engagement::battle::BattlePhase;
use crate::network::event::BattleSnapshot;
use crate::tui::components::scroll::ScrollState;
use crate::tui::components::status_dialog::{AttributeRow, StatusDialog};
use crate::tui::components::typewriter::TypewriterState;

#[derive(Debug, Clone, PartialEq)]
pub enum BattleFocus {
    Abilities,
    EntityList,
}

#[derive(Debug, Clone)]
pub struct QueuedAbilityInfo {
    pub ability_id: String,
    pub target_id: i64,
}

#[derive(Debug, Clone)]
pub struct BattleState {
    pub engagement_id: i64,
    pub snapshot: BattleSnapshot,
    pub message_log: Vec<BattleLogEntry>,
    pub log_scroll: ScrollState,
    pub selected_ability_index: usize,
    pub selected_entity_index: usize,
    pub entity_scroll: usize,
    pub focus: BattleFocus,
    pub dialog: Option<TargetDialog>,
    pub status_dialog: Option<StatusDialog>,
    pub queued_ability: Option<QueuedAbilityInfo>,
    pub reveal: Option<TypewriterState>,
    pub reveal_queue: VecDeque<usize>,
}

impl BattleState {
    pub fn new(engagement_id: i64, snapshot: BattleSnapshot) -> Self {
        Self {
            engagement_id,
            snapshot,
            message_log: Vec::new(),
            log_scroll: ScrollState::default(),
            selected_ability_index: 0,
            selected_entity_index: 0,
            entity_scroll: 0,
            focus: BattleFocus::Abilities,
            dialog: None,
            status_dialog: None,
            queued_ability: None,
            reveal: None,
            reveal_queue: VecDeque::new(),
        }
    }

    pub fn filtered_abilities(&self) -> Vec<Ability> {
        if !self.is_player_turn() {
            return vec![];
        }
        let abilities: Vec<Ability> = match &self.snapshot.phase {
            BattlePhase::DeclareAttacks { .. } => self
                .snapshot
                .available_abilities
                .iter()
                .filter(|a| a.role == AbilityRole::Attack)
                .cloned()
                .collect(),
            BattlePhase::DeclareDefense { .. } => self
                .snapshot
                .available_abilities
                .iter()
                .filter(|a| a.role == AbilityRole::Defend)
                .cloned()
                .collect(),
            _ => vec![],
        };
        if abilities.is_empty() {
            vec![Ability {
                id: "skip".to_string(),
                name: "Skip".to_string(),
                description: Description::default(),
                effects: vec![],
                engagement_types: vec![],
                costs: vec![],
                script: None,
                role: AbilityRole::Attack,
                targets: vec![],
                action_text: None,
            }]
        } else {
            abilities
        }
    }

    pub fn ability_role_label(&self) -> Option<&str> {
        if !self.is_player_turn() {
            return None;
        }
        match &self.snapshot.phase {
            BattlePhase::DeclareAttacks { .. } => Some("Attack"),
            BattlePhase::DeclareDefense { .. } => Some("Defend"),
            _ => None,
        }
    }

    pub fn select_next_ability(&mut self) {
        let len = self.filtered_abilities().len();
        if len > 0 {
            self.selected_ability_index = (self.selected_ability_index + 1) % len;
        }
    }

    pub fn select_prev_ability(&mut self) {
        let len = self.filtered_abilities().len();
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

    pub fn is_dialog_open(&self) -> bool {
        self.dialog.is_some() || self.status_dialog.is_some()
    }

    pub fn open_status_dialog(&mut self, entity_id: i64) {
        let Some(participant) = self
            .snapshot
            .participants
            .values()
            .flatten()
            .find(|p| p.id == entity_id)
        else {
            return;
        };
        let rows = participant
            .attributes
            .iter()
            .map(|a| AttributeRow {
                title: a.title.clone(),
                current: a.current,
                max: a.max,
            })
            .collect();
        self.status_dialog = Some(StatusDialog::new(participant.name.clone(), rows));
    }

    pub fn close_status_dialog(&mut self) {
        self.status_dialog = None;
    }

    pub fn is_player_turn(&self) -> bool {
        let player_faction = self.snapshot.factions.first().map(String::as_str);
        match &self.snapshot.phase {
            BattlePhase::DeclareAttacks { faction } => Some(faction.as_str()) == player_faction,
            BattlePhase::DeclareDefense { faction } => Some(faction.as_str()) != player_faction,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::network::event::{AttributeInfo, BattleSnapshot, ParticipantInfo};

    fn make_snapshot(phase: BattlePhase) -> BattleSnapshot {
        BattleSnapshot {
            factions: vec!["player".to_string(), "enemy".to_string()],
            participants: HashMap::new(),
            phase,
            turn_order: vec![],
            countdown_secs: 0,
            max_turn_secs: 30,
            available_abilities: vec![],
        }
    }

    fn make_state(phase: BattlePhase) -> BattleState {
        BattleState::new(1, make_snapshot(phase))
    }

    fn participant_with_attributes(id: i64, name: &str) -> ParticipantInfo {
        ParticipantInfo {
            id,
            name: name.to_string(),
            hp_current: 42,
            hp_max: 100,
            attributes: vec![AttributeInfo {
                id: "strength".to_string(),
                title: "Strength".to_string(),
                current: 12,
                max: 20,
            }],
        }
    }

    #[test]
    fn is_player_turn_true_when_player_faction_is_declaring_attacks() {
        assert!(
            make_state(BattlePhase::DeclareAttacks {
                faction: "player".into()
            })
            .is_player_turn()
        );
    }

    #[test]
    fn is_player_turn_false_when_other_faction_is_declaring_attacks() {
        assert!(
            !make_state(BattlePhase::DeclareAttacks {
                faction: "enemy".into()
            })
            .is_player_turn()
        );
    }

    #[test]
    fn is_player_turn_true_when_defending_against_enemy_declare() {
        // DeclareDefense { faction: "enemy" } means enemy declared; player (factions[0]) defends
        assert!(
            make_state(BattlePhase::DeclareDefense {
                faction: "enemy".into()
            })
            .is_player_turn()
        );
    }

    #[test]
    fn is_player_turn_false_when_player_declared_and_enemy_defends() {
        // DeclareDefense { faction: "player" } means player declared; enemy defends, not player
        assert!(
            !make_state(BattlePhase::DeclareDefense {
                faction: "player".into()
            })
            .is_player_turn()
        );
    }

    #[test]
    fn is_player_turn_false_during_reset_attributes() {
        assert!(
            !make_state(BattlePhase::ResetAttributes {
                faction: "player".into()
            })
            .is_player_turn()
        );
    }

    #[test]
    fn is_player_turn_false_during_resolve_abilities() {
        assert!(!make_state(BattlePhase::ResolveAbilities).is_player_turn());
    }

    #[test]
    fn is_player_turn_false_during_concluded() {
        assert!(!make_state(BattlePhase::Concluded).is_player_turn());
    }

    #[test]
    fn open_status_dialog_builds_rows_from_matching_participant() {
        let mut state = make_state(BattlePhase::ResolveAbilities);
        state.snapshot.participants.insert(
            "player".to_string(),
            vec![participant_with_attributes(1, "Hero")],
        );

        state.open_status_dialog(1);

        let dialog = state.status_dialog.expect("expected status dialog to open");
        assert_eq!(dialog.entity_name, "Hero");
        assert_eq!(dialog.rows.len(), 1);
        assert_eq!(dialog.rows[0].title, "Strength");
        assert_eq!(dialog.rows[0].current, 12);
        assert_eq!(dialog.rows[0].max, 20);
    }

    #[test]
    fn open_status_dialog_does_nothing_for_unknown_entity() {
        let mut state = make_state(BattlePhase::ResolveAbilities);

        state.open_status_dialog(99);

        assert!(state.status_dialog.is_none());
    }

    #[test]
    fn close_status_dialog_clears_it() {
        let mut state = make_state(BattlePhase::ResolveAbilities);
        state.snapshot.participants.insert(
            "player".to_string(),
            vec![participant_with_attributes(1, "Hero")],
        );
        state.open_status_dialog(1);

        state.close_status_dialog();

        assert!(state.status_dialog.is_none());
    }

    #[test]
    fn is_dialog_open_true_when_status_dialog_open() {
        let mut state = make_state(BattlePhase::ResolveAbilities);
        state.snapshot.participants.insert(
            "player".to_string(),
            vec![participant_with_attributes(1, "Hero")],
        );
        state.open_status_dialog(1);

        assert!(state.is_dialog_open());
    }

    #[test]
    fn is_dialog_open_false_when_no_dialog_open() {
        assert!(!make_state(BattlePhase::ResolveAbilities).is_dialog_open());
    }
}
