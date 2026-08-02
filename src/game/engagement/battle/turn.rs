use std::collections::HashMap;

use crate::game::component::{Ability, Attribute};
use crate::game::entity::Entity;

use super::action_queue::ActionQueue;
use super::{BattleMessage, BattlePhase, BattleTick, QueuedAbility};

struct PhaseOutput {
    messages: Vec<BattleMessage>,
    resolution_queue: Vec<QueuedAbility>,
    pending_actions: Vec<QueuedAbility>,
    completed_phase: BattlePhase,
}

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

    pub fn refund_all_costs(&self, entities: &mut HashMap<i64, Entity>) {
        self.action_queue.refund_all(entities);
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

    pub fn tick(&mut self, engagement_id: i64, max_engage_ticks: u64) -> BattleTick {
        let all_participant_ids = self.all_entity_ids();
        let output = self.advance_phase(engagement_id, max_engage_ticks);
        BattleTick {
            engagement_id,
            all_participant_ids,
            messages: output.messages,
            turn_count: self.turn_count,
            resolution_queue: output.resolution_queue,
            pending_actions: output.pending_actions,
            phase: self.turn_phase.clone(),
            completed_phase: output.completed_phase,
            factions: self.factions.clone(),
            participants: self.participants.clone(),
            ticks_in_phase: self.ticks_in_phase,
        }
    }

    /// Drives the phase state machine one step, returning messages and queued work produced by
    /// the transition. Advances `self.turn_phase` and resets `self.ticks_in_phase` as needed.
    ///
    /// Every phase except `DeclareAttacks`/`DeclareDefense` is an unconditional single-tick
    /// transition — one `tick()` call advances exactly one phase-arm. Only `DeclareAttacks` and
    /// `DeclareDefense` dwell across multiple raw ticks, waiting for player/AI submissions or a
    /// timeout. This is deliberate: walking the non-dwelling phases (e.g. `ResolveAbilities`
    /// through `VictoryCheck`, or `ResetAttributes` through `DeclareAttacks`) now costs several
    /// more raw ticks than the old 5-phase machine's equivalent single-call transitions did. None
    /// of the new phases block on player input, so this only adds a few non-waiting game-loop
    /// ticks per turn — do not "optimize" this into a multi-arm-per-call loop.
    fn advance_phase(&mut self, engagement_id: i64, max_engage_ticks: u64) -> PhaseOutput {
        let completed_phase = self.turn_phase.clone();
        let mut out = PhaseOutput {
            messages: Vec::new(),
            resolution_queue: Vec::new(),
            pending_actions: Vec::new(),
            completed_phase: completed_phase.clone(),
        };
        match completed_phase {
            BattlePhase::ResetAttributes { .. } => {
                self.advance_reset_attributes(&mut out, engagement_id)
            }
            BattlePhase::AnnounceState { .. } => self.advance_announce_state(&mut out),
            BattlePhase::ApplyEffects { .. } => self.advance_apply_effects(&mut out),
            BattlePhase::DeclareAttacks { faction } => {
                self.advance_declare_attacks(&mut out, engagement_id, faction, max_engage_ticks)
            }
            BattlePhase::DeclareDefense { .. } => {
                self.advance_declare_defense(&mut out, engagement_id, max_engage_ticks)
            }
            BattlePhase::ResolveAbilities => {
                self.advance_resolve_abilities(&mut out, engagement_id)
            }
            BattlePhase::ResolveEntityState => self.advance_resolve_entity_state(&mut out),
            BattlePhase::Cleanup => self.advance_cleanup(&mut out),
            BattlePhase::VictoryCheck => self.advance_victory_check(&mut out, engagement_id),
            BattlePhase::Concluded => {}
        }
        out
    }

    fn transition_to(&mut self, out: &mut PhaseOutput, next: BattlePhase) {
        out.messages.push(BattleMessage::PhaseChange {
            phase: next.clone(),
        });
        self.turn_phase = next;
        self.ticks_in_phase = 0;
    }

    fn advance_reset_attributes(&mut self, out: &mut PhaseOutput, engagement_id: i64) {
        self.turn_count += 1;
        tracing::info!(
            engagement_id,
            faction = %self.planning_faction(),
            turn_count = self.turn_count,
            "Starting battle turn for faction"
        );
        let next = BattlePhase::AnnounceState {
            faction: self.planning_faction().to_string(),
        };
        self.transition_to(out, next);
    }

    fn advance_announce_state(&mut self, out: &mut PhaseOutput) {
        let next = BattlePhase::ApplyEffects {
            faction: self.planning_faction().to_string(),
        };
        self.transition_to(out, next);
    }

    fn advance_apply_effects(&mut self, out: &mut PhaseOutput) {
        let next = BattlePhase::DeclareAttacks {
            faction: self.planning_faction().to_string(),
        };
        self.transition_to(out, next);
    }

    fn advance_declare_attacks(
        &mut self,
        out: &mut PhaseOutput,
        engagement_id: i64,
        faction: String,
        max_engage_ticks: u64,
    ) {
        self.ticks_in_phase += 1;
        let planning_ids = self.planning_ids();
        let all_submitted = self.action_queue.all_submitted(&planning_ids);
        if all_submitted || self.ticks_in_phase >= max_engage_ticks {
            out.pending_actions = self.action_queue.snapshot();
            tracing::info!(
                engagement_id,
                faction = %faction,
                planning_count = planning_ids.len(),
                queued_count = self.action_queue.queued_count(&planning_ids),
                timed_out = !all_submitted,
                "DeclareAttacks phase complete"
            );
            self.transition_to(out, BattlePhase::DeclareDefense { faction });
        }
    }

    fn advance_declare_defense(
        &mut self,
        out: &mut PhaseOutput,
        engagement_id: i64,
        max_engage_ticks: u64,
    ) {
        self.ticks_in_phase += 1;
        let responding_ids = self.responding_ids();
        let all_submitted =
            responding_ids.is_empty() || self.action_queue.all_submitted(&responding_ids);
        if all_submitted || self.ticks_in_phase >= max_engage_ticks {
            tracing::info!(
                engagement_id,
                responding_count = responding_ids.len(),
                queued_count = self.action_queue.queued_count(&responding_ids),
                timed_out = !all_submitted,
                "DeclareDefense phase complete"
            );
            self.transition_to(out, BattlePhase::ResolveAbilities);
        }
    }

    fn advance_resolve_abilities(&mut self, out: &mut PhaseOutput, engagement_id: i64) {
        out.resolution_queue = self.action_queue.drain();
        tracing::info!(
            engagement_id,
            resolution_count = out.resolution_queue.len(),
            "Resolving queued abilities"
        );
        self.transition_to(out, BattlePhase::ResolveEntityState);
    }

    fn advance_resolve_entity_state(&mut self, out: &mut PhaseOutput) {
        self.transition_to(out, BattlePhase::Cleanup);
    }

    fn advance_cleanup(&mut self, out: &mut PhaseOutput) {
        self.action_queue.clear_turn_state();
        self.transition_to(out, BattlePhase::VictoryCheck);
    }

    fn advance_victory_check(&mut self, out: &mut PhaseOutput, engagement_id: i64) {
        if self.surviving_faction_count() <= 1 {
            self.log_battle_concluded(engagement_id);
            self.transition_to(out, BattlePhase::Concluded);
            return;
        }
        if !self.factions.is_empty() {
            self.planning_faction_index = (self.planning_faction_index + 1) % self.factions.len();
        }
        self.log_battle_continuing(engagement_id);
        let next = BattlePhase::ResetAttributes {
            faction: self.planning_faction().to_string(),
        };
        self.transition_to(out, next);
    }

    fn log_battle_concluded(&self, engagement_id: i64) {
        tracing::info!(
            engagement_id,
            survivors = self.surviving_faction_count(),
            "Battle concluded"
        );
    }

    fn log_battle_continuing(&self, engagement_id: i64) {
        tracing::info!(
            engagement_id,
            next_faction = %self.planning_faction(),
            survivor_factions = self.surviving_faction_count(),
            "Battle continuing to next faction"
        );
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
        // ResetAttributes -> AnnounceState -> ApplyEffects -> DeclareAttacks (3 calls)
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
        // DeclareAttacks(timeout) -> DeclareDefense -> ResolveAbilities -> ResolveEntityState
        // -> Cleanup -> VictoryCheck -> ResetAttributes{enemy} (6 calls)
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
        eng.tick(1, 1); // ResetAttributes{enemy} -> AnnounceState{enemy}, turn_count becomes 2
        assert_eq!(eng.turn_count, 2);
    }

    #[test]
    fn tick_declare_attacks_waits_for_timeout() {
        let mut eng = make_engagement();
        for _ in 0..3 {
            eng.tick(1, 30); // -> DeclareAttacks{player}
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
            eng.tick(1, 1); // -> DeclareAttacks{player}
        }
        let tick = eng.tick(1, 1); // timeout -> DeclareDefense{player}
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
            eng.tick(1, 1); // -> DeclareDefense{player}
        }
        let tick = eng.tick(1, 1); // timeout -> ResolveAbilities
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
            eng.tick(1, 30); // -> DeclareAttacks{player}
        }
        let attrs = HashMap::new();
        eng.queue_ability(1, test_ability(), 2, &attrs);
        eng.tick(1, 1); // all submitted -> DeclareDefense{player}
        eng.tick(1, 1); // timeout -> ResolveAbilities
        let tick = eng.tick(1, 1); // ResolveAbilities -> ResolveEntityState
        assert_eq!(eng.turn_phase, BattlePhase::ResolveEntityState);
        assert_eq!(tick.resolution_queue.len(), 1);
        assert_eq!(tick.resolution_queue[0].caster_id, 1);
    }

    #[test]
    fn tick_cleanup_clears_skip_and_cost_state() {
        let mut eng = make_engagement();
        for _ in 0..3 {
            eng.tick(1, 30); // -> DeclareAttacks{player}
        }
        eng.skip_phase(1);
        eng.tick(1, 1); // all skipped -> DeclareDefense
        eng.tick(1, 1); // timeout -> ResolveAbilities
        eng.tick(1, 1); // ResolveAbilities -> ResolveEntityState
        eng.tick(1, 1); // ResolveEntityState -> Cleanup
        let tick = eng.tick(1, 1); // Cleanup -> VictoryCheck
        assert_eq!(eng.turn_phase, BattlePhase::VictoryCheck);
        assert_eq!(tick.completed_phase, BattlePhase::Cleanup);
        assert!(eng.unacted_planning_ids().contains(&1));
    }

    #[test]
    fn tick_victory_check_transitions_to_concluded_when_one_faction_remains() {
        let mut eng = make_engagement();
        eng.remove_entity(1); // only "enemy" faction remains
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
            eng.tick(1, 30); // -> DeclareAttacks{player}
        }
        let attrs = HashMap::new();
        eng.queue_ability(1, test_ability(), 2, &attrs);
        let tick = eng.tick(1, 1); // all submitted -> DeclareDefense{player}
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
            eng.tick(1, 1); // -> DeclareAttacks{player}
        }
        let tick = eng.tick(1, 1); // timeout -> DeclareDefense{player}
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
