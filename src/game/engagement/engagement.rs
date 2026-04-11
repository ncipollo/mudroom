use std::collections::HashMap;

use crate::game::engagement::EngagementType;
use crate::game::engagement::TurnAction;
use crate::game::engagement::TurnOrder;

pub struct Engagement {
    pub id: i64,
    pub engagement_type: EngagementType,
    pub entity_ids: Vec<i64>,
    pub turn_order: TurnOrder,
    pub pending_actions: HashMap<i64, TurnAction>,
    pub ticks_on_current_turn: u64,
}

impl Engagement {
    pub fn new(id: i64, engagement_type: EngagementType, entity_ids: Vec<i64>) -> Self {
        let turn_order = TurnOrder::new(&entity_ids);
        Self {
            id,
            engagement_type,
            entity_ids,
            turn_order,
            pending_actions: HashMap::new(),
            ticks_on_current_turn: 0,
        }
    }

    /// Create a conversation engagement where only the player takes turns. The NPC entity is
    /// tracked in entity_ids for lookup but does not participate in the turn order.
    pub fn new_conversation(id: i64, player_entity_id: i64, npc_entity_id: i64) -> Self {
        let turn_order = TurnOrder::new(&[player_entity_id]);
        Self {
            id,
            engagement_type: EngagementType::Conversation,
            entity_ids: vec![player_entity_id, npc_entity_id],
            turn_order,
            pending_actions: HashMap::new(),
            ticks_on_current_turn: 0,
        }
    }

    pub fn current_entity(&self) -> Option<i64> {
        self.turn_order.current()
    }

    /// Submit a turn action for the given entity. Any entity in the engagement may submit or
    /// update their action at any time; actions are stored per-entity and resolved in turn order.
    /// Returns true if the entity is part of this engagement.
    pub fn submit_action(&mut self, entity_id: i64, action: TurnAction) -> bool {
        if self.entity_ids.contains(&entity_id) {
            self.pending_actions.insert(entity_id, action);
            true
        } else {
            false
        }
    }

    /// Returns true if the turn should advance: the current entity has submitted an action
    /// or the turn has timed out.
    pub fn should_advance(&self, max_engage_ticks: u64) -> bool {
        let current_has_action = self
            .turn_order
            .current()
            .map(|id| self.pending_actions.contains_key(&id))
            .unwrap_or(false);
        current_has_action || self.ticks_on_current_turn >= max_engage_ticks
    }

    /// Resolve and clear the current entity's pending action, reset tick counter, and advance
    /// to the next turn.
    pub fn advance_turn(&mut self) {
        if let Some(current) = self.turn_order.current() {
            self.pending_actions.remove(&current);
        }
        self.ticks_on_current_turn = 0;
        self.turn_order.advance();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engagement() -> Engagement {
        Engagement::new(1, EngagementType::Battle, vec![10, 20, 30])
    }

    #[test]
    fn new_sets_turn_order_by_entity_id() {
        let eng = make_engagement();
        assert_eq!(eng.turn_order.order(), &[10, 20, 30]);
        assert_eq!(eng.current_entity(), Some(10));
    }

    #[test]
    fn submit_action_accepted_for_current_entity() {
        let mut eng = make_engagement();
        let action = TurnAction::SendMessage {
            content: "hi".to_string(),
        };
        assert!(eng.submit_action(10, action.clone()));
        assert_eq!(eng.pending_actions.get(&10), Some(&action));
    }

    #[test]
    fn submit_action_accepted_for_off_turn_entity() {
        let mut eng = make_engagement();
        let action = TurnAction::SendMessage {
            content: "preemptive".to_string(),
        };
        assert!(eng.submit_action(20, action.clone()));
        assert_eq!(eng.pending_actions.get(&20), Some(&action));
    }

    #[test]
    fn submit_action_rejected_for_unknown_entity() {
        let mut eng = make_engagement();
        let action = TurnAction::SendMessage {
            content: "hi".to_string(),
        };
        assert!(!eng.submit_action(99, action));
        assert!(eng.pending_actions.is_empty());
    }

    #[test]
    fn should_advance_when_current_entity_has_action() {
        let mut eng = make_engagement();
        eng.pending_actions.insert(
            10,
            TurnAction::Respond {
                content: "ok".to_string(),
            },
        );
        assert!(eng.should_advance(30));
    }

    #[test]
    fn should_not_advance_when_only_off_turn_entity_has_action() {
        let mut eng = make_engagement();
        eng.pending_actions.insert(
            20,
            TurnAction::Respond {
                content: "waiting".to_string(),
            },
        );
        assert!(!eng.should_advance(30));
    }

    #[test]
    fn should_advance_on_timeout() {
        let mut eng = make_engagement();
        eng.ticks_on_current_turn = 30;
        assert!(eng.should_advance(30));
    }

    #[test]
    fn should_not_advance_before_timeout_without_action() {
        let eng = make_engagement();
        assert!(!eng.should_advance(30));
    }

    #[test]
    fn advance_turn_clears_current_action_and_moves_to_next() {
        let mut eng = make_engagement();
        eng.pending_actions.insert(
            10,
            TurnAction::Respond {
                content: "ok".to_string(),
            },
        );
        eng.pending_actions.insert(
            20,
            TurnAction::Respond {
                content: "ready".to_string(),
            },
        );
        eng.ticks_on_current_turn = 5;
        eng.advance_turn();
        assert!(!eng.pending_actions.contains_key(&10));
        assert!(eng.pending_actions.contains_key(&20));
        assert_eq!(eng.ticks_on_current_turn, 0);
        assert_eq!(eng.current_entity(), Some(20));
    }
}
