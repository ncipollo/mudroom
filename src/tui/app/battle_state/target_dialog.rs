use super::BattleState;
use crate::game::component::AbilityTargetType;
use crate::network::event::ParticipantInfo;

#[derive(Debug, Clone)]
pub struct TargetDialog {
    pub pending_ability_id: String,
    pub selected_index: usize,
    pub targets: Vec<ParticipantInfo>,
}

impl BattleState {
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
}
