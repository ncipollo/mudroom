pub mod loot;
pub mod message;
pub mod phase;
pub mod queued_ability;

use std::collections::HashMap;

use crate::game::component::{Ability, Attribute, Cost};
use crate::game::engagement::EngagementType;

pub use message::BattleMessage;
pub use phase::BattlePhase;
pub use queued_ability::QueuedAbility;

pub struct BattleTick {
    pub engagement_id: i64,
    pub all_participant_ids: Vec<i64>,
    pub messages: Vec<BattleMessage>,
    pub innate_entity_ids: Vec<i64>,
    pub resolution_queue: Vec<QueuedAbility>,
    pub phase: BattlePhase,
}

pub struct BattleEngagement {
    pub factions: Vec<String>,
    pub participants: HashMap<String, Vec<i64>>,
    pub turn_phase: BattlePhase,
    action_queue: HashMap<i64, Vec<QueuedAbility>>,
    ticks_in_phase: u64,
    pending_costs: HashMap<i64, Vec<(String, i64)>>,
}

impl BattleEngagement {
    pub fn new(factions: Vec<String>, participants: HashMap<String, Vec<i64>>) -> Self {
        Self {
            factions,
            participants,
            turn_phase: BattlePhase::InnateEffects,
            action_queue: HashMap::new(),
            ticks_in_phase: 0,
            pending_costs: HashMap::new(),
        }
    }

    pub fn all_entity_ids(&self) -> Vec<i64> {
        self.participants.values().flatten().copied().collect()
    }

    pub fn attacker_ids(&self) -> Vec<i64> {
        self.factions
            .first()
            .and_then(|f| self.participants.get(f))
            .cloned()
            .unwrap_or_default()
    }

    pub fn defender_ids(&self) -> Vec<i64> {
        let attacker_faction = self.factions.first().map(String::as_str);
        self.participants
            .iter()
            .filter(|(f, _)| Some(f.as_str()) != attacker_faction)
            .flat_map(|(_, ids)| ids.iter().copied())
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

    pub fn refund_all_costs(&self, entities: &mut HashMap<i64, crate::game::entity::Entity>) {
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
        let mut messages = Vec::new();
        let mut innate_entity_ids = Vec::new();
        let mut resolution_queue = Vec::new();

        match self.turn_phase.clone() {
            BattlePhase::InnateEffects => {
                innate_entity_ids = all_participant_ids.clone();
                messages.push(BattleMessage::PhaseChange {
                    phase: BattlePhase::AttackerPlanning,
                });
                self.turn_phase = BattlePhase::AttackerPlanning;
                self.ticks_in_phase = 0;
            }
            BattlePhase::AttackerPlanning => {
                self.ticks_in_phase += 1;
                let attacker_ids = self.attacker_ids();
                let all_submitted = attacker_ids
                    .iter()
                    .all(|id| self.action_queue.contains_key(id));
                if all_submitted || self.ticks_in_phase >= max_engage_ticks {
                    messages.push(BattleMessage::PhaseChange {
                        phase: BattlePhase::DefenderResponse,
                    });
                    self.turn_phase = BattlePhase::DefenderResponse;
                    self.ticks_in_phase = 0;
                }
            }
            BattlePhase::DefenderResponse => {
                self.ticks_in_phase += 1;
                let defender_ids = self.defender_ids();
                let all_submitted = defender_ids.is_empty()
                    || defender_ids
                        .iter()
                        .all(|id| self.action_queue.contains_key(id));
                if all_submitted || self.ticks_in_phase >= max_engage_ticks {
                    messages.push(BattleMessage::PhaseChange {
                        phase: BattlePhase::Resolution,
                    });
                    self.turn_phase = BattlePhase::Resolution;
                    self.ticks_in_phase = 0;
                }
            }
            BattlePhase::Resolution => {
                resolution_queue = self
                    .action_queue
                    .drain()
                    .flat_map(|(_, abilities)| abilities)
                    .collect();
                self.pending_costs.clear();
                messages.push(BattleMessage::PhaseChange {
                    phase: BattlePhase::InnateEffects,
                });
                self.turn_phase = BattlePhase::InnateEffects;
                self.ticks_in_phase = 0;
            }
            BattlePhase::Concluded => {}
        }

        BattleTick {
            engagement_id,
            all_participant_ids,
            messages,
            innate_entity_ids,
            resolution_queue,
            phase: self.turn_phase.clone(),
        }
    }
}

fn battle_abilities(entity: &crate::game::entity::Entity) -> Vec<Ability> {
    entity
        .innate_abilities
        .iter()
        .filter(|a| a.engagement_types.contains(&EngagementType::Battle))
        .cloned()
        .collect()
}

pub fn entity_innate_battle_abilities(entity: &crate::game::entity::Entity) -> Vec<Ability> {
    battle_abilities(entity)
}

#[cfg(test)]
mod tests {
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
    fn attacker_ids_returns_first_faction() {
        let eng = make_engagement();
        assert_eq!(eng.attacker_ids(), vec![1]);
    }

    #[test]
    fn defender_ids_returns_non_first_factions() {
        let eng = make_engagement();
        let mut ids = eng.defender_ids();
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
    fn tick_innate_effects_transitions_to_attacker_planning() {
        let mut eng = make_engagement();
        let tick = eng.tick(1, 30);
        assert_eq!(eng.turn_phase, BattlePhase::AttackerPlanning);
        assert!(!tick.innate_entity_ids.is_empty());
        assert!(tick.messages.iter().any(|m| matches!(
            m,
            BattleMessage::PhaseChange {
                phase: BattlePhase::AttackerPlanning
            }
        )));
    }

    #[test]
    fn tick_attacker_planning_waits_for_timeout() {
        let mut eng = make_engagement();
        eng.tick(1, 30); // InnateEffects → AttackerPlanning
        let tick = eng.tick(1, 30);
        assert_eq!(eng.turn_phase, BattlePhase::AttackerPlanning);
        assert!(tick.messages.is_empty());
    }

    #[test]
    fn tick_attacker_planning_advances_on_timeout() {
        let mut eng = make_engagement();
        eng.tick(1, 1); // InnateEffects → AttackerPlanning
        let tick = eng.tick(1, 1); // timeout
        assert_eq!(eng.turn_phase, BattlePhase::DefenderResponse);
        assert!(tick.messages.iter().any(|m| matches!(
            m,
            BattleMessage::PhaseChange {
                phase: BattlePhase::DefenderResponse
            }
        )));
    }

    #[test]
    fn tick_defender_response_advances_on_timeout() {
        let mut eng = make_engagement();
        eng.tick(1, 1); // InnateEffects → AttackerPlanning
        eng.tick(1, 1); // timeout → DefenderResponse
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
        eng.tick(1, 1); // → AttackerPlanning
        eng.tick(1, 1); // → DefenderResponse
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
    fn tick_concluded_is_noop() {
        let mut eng = make_engagement();
        eng.turn_phase = BattlePhase::Concluded;
        let tick = eng.tick(1, 30);
        assert_eq!(eng.turn_phase, BattlePhase::Concluded);
        assert!(tick.messages.is_empty());
    }

    #[test]
    fn queue_ability_rejects_without_sufficient_resources() {
        use crate::game::component::effect::{Effect, EffectDescription, EffectType, TriggerInfo};

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
            }],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![Cost::Resource {
                resource_id: "mp".to_string(),
                amount: 10,
            }],
            modifiers: vec![],
        };
        let attrs: HashMap<String, Attribute> = HashMap::new(); // no mp
        assert!(!eng.queue_ability(1, ability, 2, &attrs));
    }
}
