use std::collections::HashMap;

use crate::game::component::{Ability, Attribute, Cost};
use crate::game::entity::Entity;

use super::{BattleMessage, BattlePhase, BattleTick, QueuedAbility};

struct PhaseOutput {
    messages: Vec<BattleMessage>,
    resolution_queue: Vec<QueuedAbility>,
    pending_actions: Vec<QueuedAbility>,
}

pub struct BattleEngagement {
    pub factions: Vec<String>,
    pub participants: HashMap<String, Vec<i64>>,
    pub turn_phase: BattlePhase,
    action_queue: HashMap<i64, Vec<QueuedAbility>>,
    ticks_in_phase: u64,
    turn_count: u64,
    pending_costs: HashMap<i64, Vec<(String, i64)>>,
    planning_faction_index: usize,
}

impl BattleEngagement {
    pub fn new(factions: Vec<String>, participants: HashMap<String, Vec<i64>>) -> Self {
        Self {
            factions,
            participants,
            turn_phase: BattlePhase::InnateEffects,
            action_queue: HashMap::new(),
            ticks_in_phase: 0,
            turn_count: 0,
            pending_costs: HashMap::new(),
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
        self.planning_ids()
            .into_iter()
            .filter(|id| !self.action_queue.contains_key(id))
            .collect()
    }

    pub fn unacted_responding_ids(&self) -> Vec<i64> {
        self.responding_ids()
            .into_iter()
            .filter(|id| !self.action_queue.contains_key(id))
            .collect()
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
        for cost in &ability.costs {
            let Cost::Resource {
                resource_id,
                amount,
            } = cost;
            match entity_attrs.get(resource_id) {
                Some(attr) if attr.current_value >= *amount => {}
                _ => return false,
            }
        }
        let tracked: Vec<(String, i64)> = ability
            .costs
            .iter()
            .map(|c| {
                let Cost::Resource {
                    resource_id,
                    amount,
                } = c;
                (resource_id.clone(), *amount)
            })
            .collect();
        if !tracked.is_empty() {
            self.pending_costs
                .entry(caster_id)
                .or_default()
                .extend(tracked);
        }
        self.action_queue
            .entry(caster_id)
            .or_default()
            .push(QueuedAbility {
                caster_id,
                ability,
                target_id,
            });
        true
    }

    pub fn refund_all_costs(&self, entities: &mut HashMap<i64, Entity>) {
        for (entity_id, costs) in &self.pending_costs {
            if let Some(entity) = entities.get_mut(entity_id) {
                for (attr_id, amount) in costs {
                    if let Some(attr) = entity.attributes.get_mut(attr_id) {
                        attr.current_value = (attr.current_value + amount).min(attr.max_value);
                    }
                }
            }
        }
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
        self.action_queue.remove(&entity_id);
        self.pending_costs.remove(&entity_id);
    }

    pub fn surviving_faction_count(&self) -> usize {
        self.participants
            .values()
            .filter(|ids| !ids.is_empty())
            .count()
    }

    pub fn tick(&mut self, engagement_id: i64, max_engage_ticks: u64) -> BattleTick {
        let all_participant_ids = self.all_entity_ids();
        let output = self.advance_phase(max_engage_ticks, &all_participant_ids);
        BattleTick {
            engagement_id,
            all_participant_ids,
            messages: output.messages,
            turn_count: self.turn_count,
            resolution_queue: output.resolution_queue,
            pending_actions: output.pending_actions,
            phase: self.turn_phase.clone(),
            factions: self.factions.clone(),
            participants: self.participants.clone(),
            ticks_in_phase: self.ticks_in_phase,
        }
    }

    /// Drives the phase state machine one step, returning messages and queued work produced by
    /// the transition. Advances `self.turn_phase` and resets `self.ticks_in_phase` as needed.
    fn advance_phase(&mut self, max_engage_ticks: u64, _all_ids: &[i64]) -> PhaseOutput {
        let mut out = PhaseOutput {
            messages: Vec::new(),
            resolution_queue: Vec::new(),
            pending_actions: Vec::new(),
        };
        match self.turn_phase.clone() {
            BattlePhase::InnateEffects => {
                self.turn_count += 1;
                let next = BattlePhase::Planning {
                    faction: self.planning_faction().to_string(),
                };
                out.messages.push(BattleMessage::PhaseChange {
                    phase: next.clone(),
                });
                self.turn_phase = next;
                self.ticks_in_phase = 0;
            }
            BattlePhase::Planning { faction } => {
                self.ticks_in_phase += 1;
                let all_submitted = self
                    .planning_ids()
                    .iter()
                    .all(|id| self.action_queue.contains_key(id));
                if all_submitted || self.ticks_in_phase >= max_engage_ticks {
                    let next = BattlePhase::Response { faction };
                    out.messages.push(BattleMessage::PhaseChange {
                        phase: next.clone(),
                    });
                    out.pending_actions = self.action_queue.values().flatten().cloned().collect();
                    self.turn_phase = next;
                    self.ticks_in_phase = 0;
                }
            }
            BattlePhase::Response { .. } => {
                self.ticks_in_phase += 1;
                let responding_ids = self.responding_ids();
                let all_submitted = responding_ids.is_empty()
                    || responding_ids
                        .iter()
                        .all(|id| self.action_queue.contains_key(id));
                if all_submitted || self.ticks_in_phase >= max_engage_ticks {
                    out.messages.push(BattleMessage::PhaseChange {
                        phase: BattlePhase::Resolution,
                    });
                    self.turn_phase = BattlePhase::Resolution;
                    self.ticks_in_phase = 0;
                }
            }
            BattlePhase::Resolution => {
                out.resolution_queue = self
                    .action_queue
                    .drain()
                    .flat_map(|(_, abilities)| abilities)
                    .collect();
                self.pending_costs.clear();
                if !self.factions.is_empty() {
                    self.planning_faction_index =
                        (self.planning_faction_index + 1) % self.factions.len();
                }
                out.messages.push(BattleMessage::PhaseChange {
                    phase: BattlePhase::InnateEffects,
                });
                self.turn_phase = BattlePhase::InnateEffects;
                self.ticks_in_phase = 0;
            }
            BattlePhase::Concluded => {}
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use crate::game::component::AbilityRole;
    use crate::game::component::effect::{
        Effect, EffectDescription, EffectScope, EffectType, TriggerInfo,
    };
    use crate::game::component::{Ability, Attribute, Cost};
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

    #[test]
    fn new_starts_in_innate_effects_phase() {
        let eng = make_engagement();
        assert_eq!(eng.turn_phase, BattlePhase::InnateEffects);
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
    fn tick_innate_effects_transitions_to_planning() {
        let mut eng = make_engagement();
        let tick = eng.tick(1, 30);
        assert_eq!(
            eng.turn_phase,
            BattlePhase::Planning {
                faction: "player".into()
            }
        );
        assert_eq!(tick.turn_count, 1);
        assert!(tick.messages.iter().any(|m| matches!(
            m,
            BattleMessage::PhaseChange {
                phase: BattlePhase::Planning { .. }
            }
        )));
    }

    #[test]
    fn turn_count_increments_each_innate_effects_phase() {
        let mut eng = make_engagement();
        assert_eq!(eng.turn_count, 0);
        eng.tick(1, 1); // InnateEffects → Planning (turn_count becomes 1)
        assert_eq!(eng.turn_count, 1);
        eng.tick(1, 1); // Planning → Response
        eng.tick(1, 1); // Response → Resolution
        eng.tick(1, 1); // Resolution → InnateEffects (still 1, InnateEffects hasn't fired yet)
        assert_eq!(eng.turn_count, 1);
        eng.tick(1, 1); // InnateEffects fires again → Planning (turn_count becomes 2)
        assert_eq!(eng.turn_count, 2);
    }

    #[test]
    fn tick_planning_waits_for_timeout() {
        let mut eng = make_engagement();
        eng.tick(1, 30); // InnateEffects → Planning
        let tick = eng.tick(1, 30);
        assert_eq!(
            eng.turn_phase,
            BattlePhase::Planning {
                faction: "player".into()
            }
        );
        assert!(tick.messages.is_empty());
    }

    #[test]
    fn tick_planning_advances_on_timeout() {
        let mut eng = make_engagement();
        eng.tick(1, 1); // InnateEffects → Planning{player}
        let tick = eng.tick(1, 1); // timeout → Response{player}
        assert_eq!(
            eng.turn_phase,
            BattlePhase::Response {
                faction: "player".into()
            }
        );
        assert!(tick.messages.iter().any(|m| matches!(
            m,
            BattleMessage::PhaseChange {
                phase: BattlePhase::Response { .. }
            }
        )));
    }

    #[test]
    fn tick_response_advances_on_timeout() {
        let mut eng = make_engagement();
        eng.tick(1, 1); // InnateEffects → Planning
        eng.tick(1, 1); // timeout → Response
        let tick = eng.tick(1, 1); // timeout → Resolution
        assert_eq!(eng.turn_phase, BattlePhase::Resolution);
        assert!(tick.messages.iter().any(|m| matches!(
            m,
            BattleMessage::PhaseChange {
                phase: BattlePhase::Resolution
            }
        )));
    }

    #[test]
    fn tick_resolution_drains_queue_and_resets() {
        let mut eng = make_engagement();
        eng.tick(1, 1); // → Planning
        eng.tick(1, 1); // → Response
        eng.tick(1, 1); // → Resolution
        let tick = eng.tick(1, 1); // Resolution → InnateEffects
        assert_eq!(eng.turn_phase, BattlePhase::InnateEffects);
        assert!(tick.messages.iter().any(|m| matches!(
            m,
            BattleMessage::PhaseChange {
                phase: BattlePhase::InnateEffects
            }
        )));
    }

    #[test]
    fn tick_resolution_advances_planning_faction_index() {
        let mut eng = make_engagement();
        assert_eq!(eng.planning_faction_index, 0);
        eng.tick(1, 1); // → Planning{player}
        eng.tick(1, 1); // → Response{player}
        eng.tick(1, 1); // → Resolution
        eng.tick(1, 1); // Resolution → InnateEffects (index advances to 1)
        assert_eq!(eng.planning_faction_index, 1);
        let tick = eng.tick(1, 1); // InnateEffects → Planning{enemy}
        assert_eq!(
            eng.turn_phase,
            BattlePhase::Planning {
                faction: "enemy".into()
            }
        );
        assert!(tick.messages.iter().any(|m| matches!(
            m,
            BattleMessage::PhaseChange {
                phase: BattlePhase::Planning { .. }
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
    fn tick_planning_to_response_includes_pending_actions() {
        let mut eng = make_engagement();
        eng.tick(1, 30); // InnateEffects → Planning{player}
        let ability = Ability {
            id: "slash".to_string(),
            name: "Slash".to_string(),
            description: None,
            effects: vec![],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![],
            modifiers: vec![],
            role: AbilityRole::Attack,
            targets: vec![],
        };
        let attrs = HashMap::new();
        eng.queue_ability(1, ability.clone(), 2, &attrs);
        let tick = eng.tick(1, 1); // timeout → Response{player}
        assert_eq!(
            eng.turn_phase,
            BattlePhase::Response {
                faction: "player".into()
            }
        );
        assert_eq!(tick.pending_actions.len(), 1);
        assert_eq!(tick.pending_actions[0].caster_id, 1);
        assert_eq!(tick.pending_actions[0].target_id, 2);
    }

    #[test]
    fn tick_planning_to_response_empty_pending_actions_when_no_queued() {
        let mut eng = make_engagement();
        eng.tick(1, 1); // InnateEffects → Planning
        let tick = eng.tick(1, 1); // timeout → Response
        assert_eq!(
            eng.turn_phase,
            BattlePhase::Response {
                faction: "player".into()
            }
        );
        assert!(tick.pending_actions.is_empty());
    }

    #[test]
    fn queue_ability_rejects_without_sufficient_resources() {
        let mut eng = make_engagement();
        let ability = Ability {
            id: "fireball".to_string(),
            name: "Fireball".to_string(),
            description: None,
            effects: vec![Effect {
                name: "fire_damage".to_string(),
                effect_type: EffectType::AttributeUpdate {
                    attribute_id: "hp".to_string(),
                    value: -20,
                },
                trigger_info: TriggerInfo::Once,
                description: EffectDescription::default(),
                scope: EffectScope::default(),
            }],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![Cost::Resource {
                resource_id: "mp".to_string(),
                amount: 10,
            }],
            modifiers: vec![],
            role: AbilityRole::Attack,
            targets: vec![],
        };
        let attrs: HashMap<String, Attribute> = HashMap::new(); // no mp
        assert!(!eng.queue_ability(1, ability, 2, &attrs));
    }
}
