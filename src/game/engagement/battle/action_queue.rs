use std::collections::{HashMap, HashSet};

use crate::game::component::{Ability, Attribute, Cost};
use crate::game::entity::character::Character;

use super::QueuedAbility;

/// Tracks queued abilities, per-turn skip flags, and pending resource costs for a battle.
/// Owned by `BattleEngagement` but kept separate so the state machine stays phase-sequencing only.
#[derive(Default)]
pub struct ActionQueue {
    queued: HashMap<i64, QueuedAbility>,
    skipped_ids: HashSet<i64>,
    pending_costs: HashMap<i64, Vec<(String, i64)>>,
}

impl ActionQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue an ability for the caster targeting the given character. Validates and tracks resource
    /// costs for potential refund. Returns false if the caster lacks sufficient resources.
    pub fn queue(
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
        self.pending_costs.remove(&caster_id);
        if !tracked.is_empty() {
            self.pending_costs.insert(caster_id, tracked);
        }
        self.queued.insert(
            caster_id,
            QueuedAbility {
                caster_id,
                ability,
                target_id,
            },
        );
        true
    }

    pub fn skip(&mut self, entity_id: i64) {
        self.skipped_ids.insert(entity_id);
    }

    pub fn all_submitted(&self, ids: &[i64]) -> bool {
        ids.iter()
            .all(|id| self.queued.contains_key(id) || self.skipped_ids.contains(id))
    }

    pub fn unacted(&self, ids: &[i64]) -> Vec<i64> {
        ids.iter()
            .copied()
            .filter(|id| !self.queued.contains_key(id) && !self.skipped_ids.contains(id))
            .collect()
    }

    pub fn queued_count(&self, entity_ids: &[i64]) -> usize {
        entity_ids
            .iter()
            .filter(|id| self.queued.contains_key(id))
            .count()
    }

    /// Read the currently queued abilities without draining them.
    pub fn snapshot(&self) -> Vec<QueuedAbility> {
        self.queued.values().cloned().collect()
    }

    /// Take and clear all queued abilities.
    pub fn drain(&mut self) -> Vec<QueuedAbility> {
        self.queued.drain().map(|(_, qa)| qa).collect()
    }

    /// Clear per-turn skip flags and pending costs, in preparation for the next faction's turn.
    pub fn clear_turn_state(&mut self) {
        self.skipped_ids.clear();
        self.pending_costs.clear();
    }

    pub fn refund_all(&self, entities: &mut HashMap<i64, Character>) {
        for (entity_id, costs) in &self.pending_costs {
            if let Some(character) = entities.get_mut(entity_id) {
                for (attr_id, amount) in costs {
                    if let Some(attr) = character.attributes.get_mut(attr_id) {
                        attr.current_value = (attr.current_value + amount).min(attr.max_value);
                    }
                }
            }
        }
    }

    pub fn remove_entity(&mut self, entity_id: i64) {
        self.queued.remove(&entity_id);
        self.skipped_ids.remove(&entity_id);
        self.pending_costs.remove(&entity_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::AbilityRole;
    use crate::game::component::Description;
    use crate::game::component::Location;
    use crate::game::component::effect::{
        Effect, EffectDescription, EffectScope, EffectType, TriggerInfo,
    };
    use crate::game::engagement::EngagementType;
    use crate::game::entity::character::CharacterType;

    fn attack_ability(cost: Option<(&str, i64)>) -> Ability {
        Ability {
            id: "slash".to_string(),
            name: "Slash".to_string(),
            description: Description::default(),
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
            costs: cost
                .map(|(resource_id, amount)| {
                    vec![Cost::Resource {
                        resource_id: resource_id.to_string(),
                        amount,
                    }]
                })
                .unwrap_or_default(),
            modifiers: vec![],
            role: AbilityRole::Attack,
            targets: vec![],
            action_text: None,
        }
    }

    #[test]
    fn queue_rejects_without_sufficient_resources() {
        let mut queue = ActionQueue::new();
        let ability = attack_ability(Some(("mp", 10)));
        let attrs: HashMap<String, Attribute> = HashMap::new(); // no mp
        assert!(!queue.queue(1, ability, 2, &attrs));
    }

    #[test]
    fn queue_accepts_with_sufficient_resources() {
        let mut queue = ActionQueue::new();
        let ability = attack_ability(Some(("mp", 10)));
        let mut attrs = HashMap::new();
        attrs.insert(
            "mp".to_string(),
            Attribute::new("mp".to_string(), 0, 20, 20),
        );
        assert!(queue.queue(1, ability, 2, &attrs));
        assert_eq!(queue.snapshot().len(), 1);
    }

    #[test]
    fn all_submitted_true_when_queued_or_skipped() {
        let mut queue = ActionQueue::new();
        let attrs = HashMap::new();
        queue.queue(1, attack_ability(None), 2, &attrs);
        queue.skip(3);
        assert!(queue.all_submitted(&[1, 3]));
        assert!(!queue.all_submitted(&[1, 3, 4]));
    }

    #[test]
    fn unacted_excludes_queued_and_skipped() {
        let mut queue = ActionQueue::new();
        let attrs = HashMap::new();
        queue.queue(1, attack_ability(None), 2, &attrs);
        queue.skip(3);
        assert_eq!(queue.unacted(&[1, 3, 4]), vec![4]);
    }

    #[test]
    fn queued_count_counts_only_queued_ids() {
        let mut queue = ActionQueue::new();
        let attrs = HashMap::new();
        queue.queue(1, attack_ability(None), 2, &attrs);
        queue.skip(3);
        assert_eq!(queue.queued_count(&[1, 3, 4]), 1);
    }

    #[test]
    fn drain_empties_queue_and_returns_entries() {
        let mut queue = ActionQueue::new();
        let attrs = HashMap::new();
        queue.queue(1, attack_ability(None), 2, &attrs);
        let drained = queue.drain();
        assert_eq!(drained.len(), 1);
        assert!(queue.snapshot().is_empty());
    }

    #[test]
    fn clear_turn_state_clears_skips_and_costs() {
        let mut queue = ActionQueue::new();
        let mut attrs = HashMap::new();
        attrs.insert(
            "mp".to_string(),
            Attribute::new("mp".to_string(), 0, 20, 20),
        );
        queue.queue(1, attack_ability(Some(("mp", 10))), 2, &attrs);
        queue.skip(3);
        assert!(queue.all_submitted(&[3]));

        queue.clear_turn_state();

        assert!(!queue.all_submitted(&[3]));
        assert_eq!(queue.unacted(&[3]), vec![3]);

        let mut entities = HashMap::new();
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character.attributes.insert(
            "mp".to_string(),
            Attribute::new("mp".to_string(), 0, 20, 10),
        );
        entities.insert(1, character);
        queue.refund_all(&mut entities); // pending_costs cleared, so no refund happens
        assert_eq!(entities[&1].attributes["mp"].current_value, 10);
    }

    #[test]
    fn refund_all_restores_costs_capped_at_max() {
        let mut queue = ActionQueue::new();
        let mut attrs = HashMap::new();
        attrs.insert(
            "mp".to_string(),
            Attribute::new("mp".to_string(), 0, 20, 20),
        );
        queue.queue(1, attack_ability(Some(("mp", 10))), 2, &attrs);

        let mut entities = HashMap::new();
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character.attributes.insert(
            "mp".to_string(),
            Attribute::new("mp".to_string(), 0, 20, 10),
        );
        entities.insert(1, character);

        queue.refund_all(&mut entities);
        assert_eq!(entities[&1].attributes["mp"].current_value, 20);
    }

    #[test]
    fn remove_entity_clears_queue_skip_and_costs() {
        let mut queue = ActionQueue::new();
        let mut attrs = HashMap::new();
        attrs.insert(
            "mp".to_string(),
            Attribute::new("mp".to_string(), 0, 20, 20),
        );
        queue.queue(1, attack_ability(Some(("mp", 10))), 2, &attrs);
        queue.skip(3);
        queue.remove_entity(1);
        queue.remove_entity(3);
        assert!(queue.snapshot().is_empty());
        assert_eq!(queue.unacted(&[1, 3]), vec![1, 3]);
    }

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }
}
