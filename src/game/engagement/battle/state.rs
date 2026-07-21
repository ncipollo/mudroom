use std::collections::{HashMap, HashSet};

use crate::game::component::{Ability, Attribute, Cost, ResetCondition};
use crate::game::config::AttributeConfig;
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
    action_queue: HashMap<i64, QueuedAbility>,
    skipped_ids: HashSet<i64>,
    ticks_in_phase: u64,
    turn_count: u64,
    pending_costs: HashMap<i64, Vec<(String, i64)>>,
    planning_faction_index: usize,
    pub pending_entity_attributes: HashMap<i64, HashMap<String, Attribute>>,
}

impl BattleEngagement {
    pub fn new(factions: Vec<String>, participants: HashMap<String, Vec<i64>>) -> Self {
        Self {
            factions,
            participants,
            turn_phase: BattlePhase::InnateEffects,
            action_queue: HashMap::new(),
            skipped_ids: HashSet::new(),
            ticks_in_phase: 0,
            turn_count: 0,
            pending_costs: HashMap::new(),
            planning_faction_index: 0,
            pending_entity_attributes: HashMap::new(),
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
            .filter(|id| !self.action_queue.contains_key(id) && !self.skipped_ids.contains(id))
            .collect()
    }

    pub fn unacted_responding_ids(&self) -> Vec<i64> {
        self.responding_ids()
            .into_iter()
            .filter(|id| !self.action_queue.contains_key(id) && !self.skipped_ids.contains(id))
            .collect()
    }

    pub fn skip_phase(&mut self, entity_id: i64) {
        self.skipped_ids.insert(entity_id);
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
            let current = self
                .pending_entity_attributes
                .get(&caster_id)
                .and_then(|attrs| attrs.get(resource_id))
                .or_else(|| entity_attrs.get(resource_id));
            match current {
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
        self.pending_costs.remove(&caster_id);
        if !tracked.is_empty() {
            self.pending_costs.insert(caster_id, tracked);
        }
        self.action_queue.insert(
            caster_id,
            QueuedAbility {
                caster_id,
                ability,
                target_id,
            },
        );
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
        self.skipped_ids.remove(&entity_id);
        self.pending_costs.remove(&entity_id);
        self.pending_entity_attributes.remove(&entity_id);
    }

    pub fn surviving_faction_count(&self) -> usize {
        self.participants
            .values()
            .filter(|ids| !ids.is_empty())
            .count()
    }

    pub fn tick(
        &mut self,
        engagement_id: i64,
        max_engage_ticks: u64,
        entities: &HashMap<i64, Entity>,
        attribute_config: &AttributeConfig,
    ) -> BattleTick {
        let all_participant_ids = self.all_entity_ids();
        let output = self.advance_phase(max_engage_ticks, entities, attribute_config);
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
    fn advance_phase(
        &mut self,
        max_engage_ticks: u64,
        entities: &HashMap<i64, Entity>,
        attribute_config: &AttributeConfig,
    ) -> PhaseOutput {
        let mut out = PhaseOutput {
            messages: Vec::new(),
            resolution_queue: Vec::new(),
            pending_actions: Vec::new(),
        };
        match self.turn_phase.clone() {
            BattlePhase::InnateEffects => {
                self.turn_count += 1;
                self.refresh_pending_attributes(entities, attribute_config);
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
                    .all(|id| self.action_queue.contains_key(id) || self.skipped_ids.contains(id));
                if all_submitted || self.ticks_in_phase >= max_engage_ticks {
                    let next = BattlePhase::Response { faction };
                    out.messages.push(BattleMessage::PhaseChange {
                        phase: next.clone(),
                    });
                    out.pending_actions = self.action_queue.values().cloned().collect();
                    self.turn_phase = next;
                    self.ticks_in_phase = 0;
                }
            }
            BattlePhase::Response { .. } => {
                self.ticks_in_phase += 1;
                let responding_ids = self.responding_ids();
                let all_submitted = responding_ids.is_empty()
                    || responding_ids.iter().all(|id| {
                        self.action_queue.contains_key(id) || self.skipped_ids.contains(id)
                    });
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
                    .map(|(_, ability)| ability)
                    .collect();
                self.skipped_ids.clear();
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

    /// Refreshes the pending attribute working copy for every participant at the start of a
    /// faction turn: `EachEngagementTurn` attributes always reset to the actual value;
    /// `EndOfEngagement` and `Never` attributes are only seeded from actual the first time (so
    /// in-progress changes from earlier turns aren't clobbered).
    fn refresh_pending_attributes(
        &mut self,
        entities: &HashMap<i64, Entity>,
        attribute_config: &AttributeConfig,
    ) {
        for entity_id in self.all_entity_ids() {
            let Some(entity) = entities.get(&entity_id) else {
                continue;
            };
            let pending = self.pending_entity_attributes.entry(entity_id).or_default();
            for def in &attribute_config.attributes {
                let Some(actual) = entity.attributes.get(&def.id) else {
                    continue;
                };
                match def.reset_condition {
                    ResetCondition::EachEngagementTurn => {
                        pending.insert(def.id.clone(), actual.clone());
                    }
                    ResetCondition::EndOfEngagement | ResetCondition::Never => {
                        pending
                            .entry(def.id.clone())
                            .or_insert_with(|| actual.clone());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::game::component::AbilityRole;
    use crate::game::component::effect::{
        Effect, EffectDescription, EffectScope, EffectType, TriggerInfo,
    };
    use crate::game::component::location::Location;
    use crate::game::component::{
        Ability, Attribute, AttributeCategory, AttributeDefinition, AttributeType, Cost,
    };
    use crate::game::engagement::EngagementType;
    use crate::game::engagement::battle::{BattleMessage, BattlePhase};
    use crate::game::entity::EntityType;
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
        let tick = eng.tick(1, 30, &HashMap::new(), &AttributeConfig::default_config());
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
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // InnateEffects → Planning (turn_count becomes 1)
        assert_eq!(eng.turn_count, 1);
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // Planning → Response
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // Response → Resolution
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // Resolution → InnateEffects (still 1, InnateEffects hasn't fired yet)
        assert_eq!(eng.turn_count, 1);
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // InnateEffects fires again → Planning (turn_count becomes 2)
        assert_eq!(eng.turn_count, 2);
    }

    #[test]
    fn tick_planning_waits_for_timeout() {
        let mut eng = make_engagement();
        eng.tick(1, 30, &HashMap::new(), &AttributeConfig::default_config()); // InnateEffects → Planning
        let tick = eng.tick(1, 30, &HashMap::new(), &AttributeConfig::default_config());
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
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // InnateEffects → Planning{player}
        let tick = eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // timeout → Response{player}
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
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // InnateEffects → Planning
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // timeout → Response
        let tick = eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // timeout → Resolution
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
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // → Planning
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // → Response
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // → Resolution
        let tick = eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // Resolution → InnateEffects
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
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // → Planning{player}
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // → Response{player}
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // → Resolution
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // Resolution → InnateEffects (index advances to 1)
        assert_eq!(eng.planning_faction_index, 1);
        let tick = eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // InnateEffects → Planning{enemy}
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
        let tick = eng.tick(1, 30, &HashMap::new(), &AttributeConfig::default_config());
        assert_eq!(eng.turn_phase, BattlePhase::Concluded);
        assert!(tick.messages.is_empty());
    }

    #[test]
    fn tick_planning_to_response_includes_pending_actions() {
        let mut eng = make_engagement();
        eng.tick(1, 30, &HashMap::new(), &AttributeConfig::default_config()); // InnateEffects → Planning{player}
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
            action_text: None,
        };
        let attrs = HashMap::new();
        eng.queue_ability(1, ability.clone(), 2, &attrs);
        let tick = eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // timeout → Response{player}
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
        eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // InnateEffects → Planning
        let tick = eng.tick(1, 1, &HashMap::new(), &AttributeConfig::default_config()); // timeout → Response
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
            action_text: None,
        };
        let attrs: HashMap<String, Attribute> = HashMap::new(); // no mp
        assert!(!eng.queue_ability(1, ability, 2, &attrs));
    }

    fn test_attribute_config() -> AttributeConfig {
        AttributeConfig {
            attributes: vec![
                AttributeDefinition {
                    id: "hp".to_string(),
                    title: "Hit Points".to_string(),
                    description: String::new(),
                    min_value: 0,
                    max_value: 100,
                    attribute_type: AttributeType::HP,
                    attribute_category: AttributeCategory::Life,
                    reset_condition: ResetCondition::Never,
                },
                AttributeDefinition {
                    id: "speed".to_string(),
                    title: "Speed".to_string(),
                    description: String::new(),
                    min_value: 0,
                    max_value: 100,
                    attribute_type: AttributeType::Stat,
                    attribute_category: AttributeCategory::Speed,
                    reset_condition: ResetCondition::EachEngagementTurn,
                },
            ],
        }
    }

    fn entity_with_attrs(id: i64, hp: i64, speed: i64) -> Entity {
        let loc = Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        };
        let mut entity = Entity::new(id, EntityType::Player, loc);
        entity.attributes.insert(
            "hp".to_string(),
            Attribute::new("hp".to_string(), 0, 100, hp),
        );
        entity.attributes.insert(
            "speed".to_string(),
            Attribute::new("speed".to_string(), 0, 100, speed),
        );
        entity
    }

    #[test]
    fn innate_effects_resets_each_engagement_turn_attr_to_actual() {
        let mut eng = make_engagement();
        let mut entities = HashMap::new();
        entities.insert(1, entity_with_attrs(1, 100, 10));
        let config = test_attribute_config();

        // Pretend a prior in-turn effect changed pending speed away from actual.
        eng.pending_entity_attributes.insert(
            1,
            HashMap::from([(
                "speed".to_string(),
                Attribute::new("speed".to_string(), 0, 100, 99),
            )]),
        );

        eng.tick(1, 30, &entities, &config); // InnateEffects → Planning

        assert_eq!(eng.pending_entity_attributes[&1]["speed"].current_value, 10);
    }

    #[test]
    fn innate_effects_does_not_reset_never_attr() {
        let mut eng = make_engagement();
        let mut entities = HashMap::new();
        entities.insert(1, entity_with_attrs(1, 100, 10));
        let config = test_attribute_config();

        // hp already has a pending value that diverges from actual (e.g. mid-battle damage
        // not yet flushed) — the InnateEffects refresh must not clobber it.
        eng.pending_entity_attributes.insert(
            1,
            HashMap::from([(
                "hp".to_string(),
                Attribute::new("hp".to_string(), 0, 100, 50),
            )]),
        );

        eng.tick(1, 30, &entities, &config); // InnateEffects → Planning

        assert_eq!(eng.pending_entity_attributes[&1]["hp"].current_value, 50);
    }

    #[test]
    fn innate_effects_initializes_never_attr_from_actual_on_first_turn() {
        let mut eng = make_engagement();
        let mut entities = HashMap::new();
        entities.insert(1, entity_with_attrs(1, 80, 10));
        let config = test_attribute_config();

        assert!(eng.pending_entity_attributes.is_empty());

        eng.tick(1, 30, &entities, &config); // first InnateEffects → Planning

        assert_eq!(eng.pending_entity_attributes[&1]["hp"].current_value, 80);
    }
}
