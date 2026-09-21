use super::BattleEngagement;
use crate::game::engagement::battle::{BattleMessage, BattlePhase, BattleTick, QueuedAbility};

struct PhaseOutput {
    messages: Vec<BattleMessage>,
    resolution_queue: Vec<QueuedAbility>,
    pending_actions: Vec<QueuedAbility>,
    completed_phase: BattlePhase,
}

impl BattleEngagement {
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

    /// Drives the phase state machine one step. Only `DeclareAttacks`/`DeclareDefense` dwell
    /// across multiple ticks, waiting on player/AI submissions or a timeout; every other phase
    /// advances exactly one phase-arm per `tick()` call. Do not "optimize" this into a
    /// multi-arm-per-call loop — the extra non-waiting ticks vs. the old 5-phase machine are
    /// intentional and don't block on player input.
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
