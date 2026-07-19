use crate::game::component::{Ability, AbilityRole, AbilityTargetType};
use crate::game::engagement::battle::{BattleMessage, BattlePhase};
use crate::network::event::{BattleSnapshot, ParticipantInfo};

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
    pub queued_ability: Option<QueuedAbilityInfo>,
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
            queued_ability: None,
        }
    }

    pub fn filtered_abilities(&self) -> Vec<Ability> {
        if !self.is_player_turn() {
            return vec![];
        }
        let abilities: Vec<Ability> = match &self.snapshot.phase {
            BattlePhase::Planning { .. } => self
                .snapshot
                .available_abilities
                .iter()
                .filter(|a| a.role == AbilityRole::Attack)
                .cloned()
                .collect(),
            BattlePhase::Response { .. } => self
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
                description: None,
                effects: vec![],
                engagement_types: vec![],
                costs: vec![],
                modifiers: vec![],
                role: AbilityRole::Attack,
                targets: vec![],
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
            BattlePhase::Planning { .. } => Some("Attack"),
            BattlePhase::Response { .. } => Some("Defend"),
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

    pub fn open_target_dialog(&mut self, ability_id: String, target_types: Vec<AbilityTargetType>) {
        let targets = self.filter_targets(&target_types);
        self.dialog = Some(TargetDialog {
            pending_ability_id: ability_id,
            selected_index: 0,
            targets,
        });
    }

    fn filter_targets(&self, target_types: &[AbilityTargetType]) -> Vec<ParticipantInfo> {
        if target_types.is_empty() {
            return self
                .snapshot
                .participants
                .values()
                .flat_map(|infos| infos.iter().cloned())
                .collect();
        }
        let player_faction = match self.snapshot.factions.first() {
            Some(f) => f,
            None => {
                return self
                    .snapshot
                    .participants
                    .values()
                    .flat_map(|infos| infos.iter().cloned())
                    .collect();
            }
        };
        let mut targets: Vec<ParticipantInfo> = Vec::new();
        for target_type in target_types {
            match target_type {
                AbilityTargetType::SelfTarget | AbilityTargetType::Allies => {
                    if let Some(infos) = self.snapshot.participants.get(player_faction) {
                        targets.extend(infos.iter().cloned());
                    }
                }
                AbilityTargetType::Opponent => {
                    for (faction, infos) in &self.snapshot.participants {
                        if faction != player_faction {
                            targets.extend(infos.iter().cloned());
                        }
                    }
                }
            }
        }
        targets.dedup_by_key(|p| p.id);
        targets
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
        let player_faction = self.snapshot.factions.first().map(String::as_str);
        match &self.snapshot.phase {
            BattlePhase::Planning { faction } => Some(faction.as_str()) == player_faction,
            BattlePhase::Response { faction } => Some(faction.as_str()) != player_faction,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::network::event::BattleSnapshot;

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

    #[test]
    fn is_player_turn_true_when_player_faction_is_planning() {
        assert!(
            make_state(BattlePhase::Planning {
                faction: "player".into()
            })
            .is_player_turn()
        );
    }

    #[test]
    fn is_player_turn_false_when_other_faction_is_planning() {
        assert!(
            !make_state(BattlePhase::Planning {
                faction: "enemy".into()
            })
            .is_player_turn()
        );
    }

    #[test]
    fn is_player_turn_true_when_defending_against_enemy_plan() {
        // Response { faction: "enemy" } means enemy planned; player (factions[0]) is defending
        assert!(
            make_state(BattlePhase::Response {
                faction: "enemy".into()
            })
            .is_player_turn()
        );
    }

    #[test]
    fn is_player_turn_false_when_player_planned_and_enemy_defends() {
        // Response { faction: "player" } means player planned; enemy is defending, not the player
        assert!(
            !make_state(BattlePhase::Response {
                faction: "player".into()
            })
            .is_player_turn()
        );
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
