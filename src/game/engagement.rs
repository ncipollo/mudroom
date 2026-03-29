pub mod engagement_type;
pub mod engagements;
pub mod turn_action;
pub mod turn_order;

pub use engagement_type::EngagementType;
pub use engagements::Engagements;
pub use turn_action::TurnAction;
pub use turn_order::TurnOrder;

pub struct Engagement {
    pub id: i64,
    pub engagement_type: EngagementType,
    pub entity_ids: Vec<i64>,
    pub turn_order: TurnOrder,
    pub pending_action: Option<TurnAction>,
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
            pending_action: None,
            ticks_on_current_turn: 0,
        }
    }

    pub fn current_entity(&self) -> Option<i64> {
        self.turn_order.current()
    }

    /// Submit a turn action for the given entity. Returns true if it is that entity's turn
    /// and the action was accepted.
    pub fn submit_action(&mut self, entity_id: i64, action: TurnAction) -> bool {
        if self.turn_order.current() == Some(entity_id) {
            self.pending_action = Some(action);
            true
        } else {
            false
        }
    }

    /// Returns true if the turn should advance: either a pending action was submitted
    /// or the turn has timed out.
    pub fn should_advance(&self, max_engage_ticks: u64) -> bool {
        self.pending_action.is_some() || self.ticks_on_current_turn >= max_engage_ticks
    }

    /// Resolve and clear the pending action, reset tick counter, and advance to next turn.
    pub fn advance_turn(&mut self) {
        self.pending_action = None;
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
        assert_eq!(eng.pending_action, Some(action));
    }

    #[test]
    fn submit_action_rejected_for_wrong_entity() {
        let mut eng = make_engagement();
        let action = TurnAction::SendMessage {
            content: "hi".to_string(),
        };
        assert!(!eng.submit_action(20, action));
        assert!(eng.pending_action.is_none());
    }

    #[test]
    fn should_advance_with_pending_action() {
        let mut eng = make_engagement();
        eng.pending_action = Some(TurnAction::Respond {
            content: "ok".to_string(),
        });
        assert!(eng.should_advance(30));
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
    fn advance_turn_clears_state_and_moves_to_next() {
        let mut eng = make_engagement();
        eng.pending_action = Some(TurnAction::Respond {
            content: "ok".to_string(),
        });
        eng.ticks_on_current_turn = 5;
        eng.advance_turn();
        assert!(eng.pending_action.is_none());
        assert_eq!(eng.ticks_on_current_turn, 0);
        assert_eq!(eng.current_entity(), Some(20));
    }
}
