use std::collections::HashMap;

use crate::game::engagement::EngagementType;
use crate::game::engagement::TurnAction;
use crate::game::engagement::TurnOrder;
use crate::game::engagement::battle::BattleEngagement;

pub struct Engagement {
    pub id: i64,
    pub engagement_type: EngagementType,
    pub room_id: Option<String>,
    pub entity_ids: Vec<i64>,
    pub turn_order: TurnOrder,
    pub pending_actions: HashMap<i64, TurnAction>,
    pub ticks_on_current_turn: u64,
    pub battle: Option<BattleEngagement>,
}

impl Engagement {
    pub fn new(id: i64, engagement_type: EngagementType, entity_ids: Vec<i64>) -> Self {
        let turn_order = TurnOrder::new(&entity_ids);
        Self {
            id,
            engagement_type,
            room_id: None,
            entity_ids,
            turn_order,
            pending_actions: HashMap::new(),
            ticks_on_current_turn: 0,
            battle: None,
        }
    }

    pub fn new_battle(
        id: i64,
        room_id: String,
        factions: Vec<String>,
        participants: HashMap<String, Vec<i64>>,
    ) -> Self {
        let entity_ids: Vec<i64> = participants.values().flatten().copied().collect();
        let turn_order = TurnOrder::new(&entity_ids);
        let battle = BattleEngagement::new(factions, participants);
        Self {
            id,
            engagement_type: EngagementType::Battle,
            room_id: Some(room_id),
            entity_ids,
            turn_order,
            pending_actions: HashMap::new(),
            ticks_on_current_turn: 0,
            battle: Some(battle),
        }
    }

    /// Create a conversation engagement where only the player takes turns. The NPC entity is
    /// tracked in entity_ids for lookup but does not participate in the turn order.
    pub fn new_conversation(id: i64, player_entity_id: i64, npc_entity_id: i64) -> Self {
        let turn_order = TurnOrder::new(&[player_entity_id]);
        Self {
            id,
            engagement_type: EngagementType::Conversation,
            room_id: None,
            entity_ids: vec![player_entity_id, npc_entity_id],
            turn_order,
            pending_actions: HashMap::new(),
            ticks_on_current_turn: 0,
            battle: None,
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

    fn make_battle_participants() -> (Vec<String>, HashMap<String, Vec<i64>>) {
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);
        participants.insert("enemy".to_string(), vec![2]);
        (
            vec!["player".to_string(), "enemy".to_string()],
            participants,
        )
    }

    #[test]
    fn new_battle_sets_room_id_and_type() {
        let (factions, participants) = make_battle_participants();
        let eng = Engagement::new_battle(5, "room1".to_string(), factions, participants);
        assert_eq!(eng.room_id, Some("room1".to_string()));
        assert_eq!(eng.engagement_type, EngagementType::Battle);
        let mut ids = eng.entity_ids.clone();
        ids.sort();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn new_battle_initializes_battle_engagement() {
        let (factions, participants) = make_battle_participants();
        let eng = Engagement::new_battle(5, "room1".to_string(), factions, participants);
        assert!(eng.battle.is_some());
    }

    #[test]
    fn new_sets_room_id_to_none() {
        let eng = make_engagement();
        assert_eq!(eng.room_id, None);
    }

    #[test]
    fn new_conversation_sets_room_id_to_none() {
        let eng = Engagement::new_conversation(2, 10, 20);
        assert_eq!(eng.room_id, None);
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
