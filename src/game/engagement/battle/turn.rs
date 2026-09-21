mod phase;

use std::collections::HashMap;

use crate::game::component::{Ability, Attribute};
use crate::game::entity::character::Character;

use super::BattlePhase;
use super::action_queue::ActionQueue;

pub struct BattleEngagement {
    pub factions: Vec<String>,
    pub participants: HashMap<String, Vec<i64>>,
    pub turn_phase: BattlePhase,
    action_queue: ActionQueue,
    ticks_in_phase: u64,
    turn_count: u64,
    planning_faction_index: usize,
}

impl BattleEngagement {
    pub fn new(factions: Vec<String>, participants: HashMap<String, Vec<i64>>) -> Self {
        let initial_faction = factions.first().cloned().unwrap_or_default();
        Self {
            factions,
            participants,
            turn_phase: BattlePhase::ResetAttributes {
                faction: initial_faction,
            },
            action_queue: ActionQueue::new(),
            ticks_in_phase: 0,
            turn_count: 0,
            planning_faction_index: 0,
        }
    }

    pub fn all_entity_ids(&self) -> Vec<i64> {
        self.participants.values().flatten().copied().collect()
    }

    fn planning_faction(&self) -> &str {
        self.factions
            .get(self.planning_faction_index)
            .map(String::as_str)
            .unwrap_or_default()
    }

    pub fn planning_ids(&self) -> Vec<i64> {
        self.participants
            .get(self.planning_faction())
            .cloned()
            .unwrap_or_default()
    }

    pub fn responding_ids(&self) -> Vec<i64> {
        let planning = self.planning_faction();
        self.participants
            .iter()
            .filter(|(f, _)| f.as_str() != planning)
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect()
    }

    pub fn unacted_planning_ids(&self) -> Vec<i64> {
        self.action_queue.unacted(&self.planning_ids())
    }

    pub fn unacted_responding_ids(&self) -> Vec<i64> {
        self.action_queue.unacted(&self.responding_ids())
    }

    pub fn skip_phase(&mut self, entity_id: i64) {
        self.action_queue.skip(entity_id);
    }

    /// Queue an ability for the caster targeting the given entity. Validates and tracks resource
    /// costs for potential refund. Returns false if the caster lacks sufficient resources.
    pub fn queue_ability(
        &mut self,
        caster_id: i64,
        ability: Ability,
        target_id: i64,
        entity_attrs: &HashMap<String, Attribute>,
    ) -> bool {
        self.action_queue
            .queue(caster_id, ability, target_id, entity_attrs)
    }

    pub fn refund_all_costs(&self, characters: &mut HashMap<i64, Character>) {
        self.action_queue.refund_all(characters);
    }

    pub fn add_entity(&mut self, faction: &str, entity_id: i64) {
        self.participants
            .entry(faction.to_string())
            .or_default()
            .push(entity_id);
    }

    pub fn remove_entity(&mut self, entity_id: i64) {
        for ids in self.participants.values_mut() {
            ids.retain(|&id| id != entity_id);
        }
        self.action_queue.remove_entity(entity_id);
    }

    pub fn surviving_faction_count(&self) -> usize {
        self.participants
            .values()
            .filter(|ids| !ids.is_empty())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use crate::game::component::Ability;
    use crate::game::component::AbilityRole;
    use crate::game::component::Description;
    use crate::game::engagement::EngagementType;
    use crate::game::engagement::battle::{BattleMessage, BattlePhase};
    use std::collections::HashMap;

    use super::*;

    fn make_participants() -> (Vec<String>, HashMap<String, Vec<i64>>) {
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);
        participants.insert("enemy".to_string(), vec![2, 3]);
        let factions = vec!["player".to_string(), "enemy".to_string()];
        (factions, participants)
    }

    fn make_engagement() -> BattleEngagement {
        let (factions, participants) = make_participants();
        BattleEngagement::new(factions, participants)
    }

    fn test_ability() -> Ability {
        Ability {
            id: "slash".to_string(),
            name: "Slash".to_string(),
            description: Description::default(),
            effects: vec![],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![],
            modifiers: vec![],
            role: AbilityRole::Attack,
            targets: vec![],
            action_text: None,
        }
    }

    #[test]
    fn new_starts_in_reset_attributes_phase() {
        let eng = make_engagement();
        assert_eq!(
            eng.turn_phase,
            BattlePhase::ResetAttributes {
                faction: "player".into()
            }
        );
    }

    #[test]
    fn all_entity_ids_returns_all_participants() {
        let eng = make_engagement();
        let mut ids = eng.all_entity_ids();
        ids.sort();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn planning_ids_returns_current_planning_faction() {
        let eng = make_engagement();
        assert_eq!(eng.planning_ids(), vec![1]);
    }

    #[test]
    fn responding_ids_returns_non_planning_factions() {
        let eng = make_engagement();
        let mut ids = eng.responding_ids();
        ids.sort();
        assert_eq!(ids, vec![2, 3]);
    }

    #[test]
    fn remove_entity_removes_from_participants() {
        let mut eng = make_engagement();
        eng.remove_entity(2);
        let mut ids = eng.all_entity_ids();
        ids.sort();
        assert_eq!(ids, vec![1, 3]);
    }

    #[test]
    fn surviving_faction_count_counts_non_empty_factions() {
        let mut eng = make_engagement();
        assert_eq!(eng.surviving_faction_count(), 2);
        eng.remove_entity(1);
        assert_eq!(eng.surviving_faction_count(), 1);
    }

    #[test]
    fn tick_reset_attributes_transitions_to_announce_state() {
        let mut eng = make_engagement();
        let tick = eng.tick(1, 30);
        assert_eq!(
            eng.turn_phase,
            BattlePhase::AnnounceState {
                faction: "player".into()
            }
        );
        assert_eq!(tick.turn_count, 1);
        assert_eq!(
            tick.completed_phase,
            BattlePhase::ResetAttributes {
                faction: "player".into()
            }
        );
        assert!(tick.messages.iter().any(|m| matches!(
            m,
            BattleMessage::PhaseChange {
                phase: BattlePhase::AnnounceState { .. }
            }
        )));
    }

    #[test]
    fn turn_count_increments_once_per_full_faction_turn() {
        let mut eng = make_engagement();
        assert_eq!(eng.turn_count, 0);
        for _ in 0..3 {
            eng.tick(1, 1);
        }
        assert_eq!(eng.turn_count, 1);
        assert_eq!(
            eng.turn_phase,
            BattlePhase::DeclareAttacks {
                faction: "player".into()
            }
        );
        for _ in 0..6 {
            eng.tick(1, 1);
        }
        assert_eq!(
            eng.turn_phase,
            BattlePhase::ResetAttributes {
                faction: "enemy".into()
            }
        );
        assert_eq!(eng.turn_count, 1);
        eng.tick(1, 1);
        assert_eq!(eng.turn_count, 2);
    }

    #[test]
    fn tick_declare_attacks_waits_for_timeout() {
        let mut eng = make_engagement();
        for _ in 0..3 {
            eng.tick(1, 30);
        }
        let tick = eng.tick(1, 30);
        assert_eq!(
            eng.turn_phase,
            BattlePhase::DeclareAttacks {
                faction: "player".into()
            }
        );
        assert!(tick.messages.is_empty());
    }

    #[test]
    fn tick_declare_attacks_advances_on_timeout() {
        let mut eng = make_engagement();
        for _ in 0..3 {
            eng.tick(1, 1);
        }
        let tick = eng.tick(1, 1);
        assert_eq!(
            eng.turn_phase,
            BattlePhase::DeclareDefense {
                faction: "player".into()
            }
        );
        assert!(tick.messages.iter().any(|m| matches!(
            m,
            BattleMessage::PhaseChange {
                phase: BattlePhase::DeclareDefense { .. }
            }
        )));
    }

    #[test]
    fn tick_declare_defense_advances_on_timeout() {
        let mut eng = make_engagement();
        for _ in 0..4 {
            eng.tick(1, 1);
        }
        let tick = eng.tick(1, 1);
        assert_eq!(eng.turn_phase, BattlePhase::ResolveAbilities);
        assert!(tick.messages.iter().any(|m| matches!(
            m,
            BattleMessage::PhaseChange {
                phase: BattlePhase::ResolveAbilities
            }
        )));
    }

    #[test]
    fn tick_resolve_abilities_drains_action_queue() {
        let mut eng = make_engagement();
        for _ in 0..3 {
            eng.tick(1, 30);
        }
        let attrs = HashMap::new();
        eng.queue_ability(1, test_ability(), 2, &attrs);
        eng.tick(1, 1);
        eng.tick(1, 1);
        let tick = eng.tick(1, 1);
        assert_eq!(eng.turn_phase, BattlePhase::ResolveEntityState);
        assert_eq!(tick.resolution_queue.len(), 1);
        assert_eq!(tick.resolution_queue[0].caster_id, 1);
    }

    #[test]
    fn tick_cleanup_clears_skip_and_cost_state() {
        let mut eng = make_engagement();
        for _ in 0..3 {
            eng.tick(1, 30);
        }
        eng.skip_phase(1);
        eng.tick(1, 1);
        eng.tick(1, 1);
        eng.tick(1, 1);
        eng.tick(1, 1);
        let tick = eng.tick(1, 1);
        assert_eq!(eng.turn_phase, BattlePhase::VictoryCheck);
        assert_eq!(tick.completed_phase, BattlePhase::Cleanup);
        assert!(eng.unacted_planning_ids().contains(&1));
    }

    #[test]
    fn tick_victory_check_transitions_to_concluded_when_one_faction_remains() {
        let mut eng = make_engagement();
        eng.remove_entity(1);
        eng.turn_phase = BattlePhase::VictoryCheck;
        let tick = eng.tick(1, 1);
        assert_eq!(eng.turn_phase, BattlePhase::Concluded);
        assert!(tick.messages.iter().any(|m| matches!(
            m,
            BattleMessage::PhaseChange {
                phase: BattlePhase::Concluded
            }
        )));
    }

    #[test]
    fn tick_victory_check_rotates_faction_and_resets_when_battle_continues() {
        let mut eng = make_engagement();
        assert_eq!(eng.planning_faction_index, 0);
        eng.turn_phase = BattlePhase::VictoryCheck;
        let tick = eng.tick(1, 1);
        assert_eq!(eng.planning_faction_index, 1);
        assert_eq!(
            eng.turn_phase,
            BattlePhase::ResetAttributes {
                faction: "enemy".into()
            }
        );
        assert!(tick.messages.iter().any(|m| matches!(
            m,
            BattleMessage::PhaseChange {
                phase: BattlePhase::ResetAttributes { .. }
            }
        )));
    }

    #[test]
    fn tick_concluded_is_noop() {
        let mut eng = make_engagement();
        eng.turn_phase = BattlePhase::Concluded;
        let tick = eng.tick(1, 30);
        assert_eq!(eng.turn_phase, BattlePhase::Concluded);
        assert!(tick.messages.is_empty());
    }

    #[test]
    fn tick_declare_attacks_to_defense_includes_pending_actions() {
        let mut eng = make_engagement();
        for _ in 0..3 {
            eng.tick(1, 30);
        }
        let attrs = HashMap::new();
        eng.queue_ability(1, test_ability(), 2, &attrs);
        let tick = eng.tick(1, 1);
        assert_eq!(
            eng.turn_phase,
            BattlePhase::DeclareDefense {
                faction: "player".into()
            }
        );
        assert_eq!(tick.pending_actions.len(), 1);
        assert_eq!(tick.pending_actions[0].caster_id, 1);
        assert_eq!(tick.pending_actions[0].target_id, 2);
    }

    #[test]
    fn tick_declare_attacks_to_defense_empty_pending_actions_when_no_queued() {
        let mut eng = make_engagement();
        for _ in 0..3 {
            eng.tick(1, 1);
        }
        let tick = eng.tick(1, 1);
        assert_eq!(
            eng.turn_phase,
            BattlePhase::DeclareDefense {
                faction: "player".into()
            }
        );
        assert!(tick.pending_actions.is_empty());
    }

    #[test]
    fn completed_phase_matches_phase_just_processed_across_full_cycle() {
        let mut eng = make_engagement();
        let expected = [
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
                faction: "player".into(),
            },
            BattlePhase::ResolveAbilities,
            BattlePhase::ResolveEntityState,
            BattlePhase::Cleanup,
            BattlePhase::VictoryCheck,
        ];
        for expected_phase in expected {
            let tick = eng.tick(1, 1);
            assert_eq!(tick.completed_phase, expected_phase);
        }
        assert_eq!(
            eng.turn_phase,
            BattlePhase::ResetAttributes {
                faction: "enemy".into()
            }
        );
    }
}
