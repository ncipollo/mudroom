pub mod battle;
pub mod conversation;
pub mod engagement_type;
pub mod processing;
pub mod resolved_action;
pub mod turn_action;
pub mod turn_order;

pub use battle::{
    AttributeSnapshot, AttributeSnapshots, BattleAiContext, BattleEngagement, BattleMessage,
    BattlePhase, BattleTick, Battles, QueuedAbility,
};
pub use conversation::Conversations;
pub use engagement_type::EngagementType;
pub use processing::process;
pub use resolved_action::ResolvedAction;
pub use turn_action::TurnAction;
pub use turn_order::TurnOrder;

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

pub struct Engagement {
    pub id: i64,
    pub engagement_type: EngagementType,
    pub room_id: Option<String>,
    pub entity_ids: Vec<i64>,
    pub turn_order: TurnOrder,
    pub pending_actions: HashMap<i64, TurnAction>,
    pub ticks_on_current_turn: u64,
    pub battle: Option<BattleEngagement>,
    pub attribute_snapshots: AttributeSnapshots,
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
            attribute_snapshots: AttributeSnapshots::default(),
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

/// Holds all active engagements split by type. `battles` and `conversations` each own their
/// engagement maps; `Engagements` itself is a thin holder that allocates shared IDs.
pub struct Engagements {
    pub battles: Battles,
    pub conversations: Conversations,
    next_id: AtomicI64,
}

impl Engagements {
    pub fn new() -> Self {
        Self {
            battles: Battles::new(),
            conversations: Conversations::new(),
            next_id: AtomicI64::new(1),
        }
    }

    fn alloc_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    pub async fn add_battle(
        &self,
        room_id: String,
        factions: Vec<String>,
        participants: HashMap<String, Vec<i64>>,
    ) -> i64 {
        let id = self.alloc_id();
        self.battles.add(id, room_id, factions, participants).await;
        id
    }

    pub async fn add_conversation(&self, player_entity_id: i64, npc_entity_id: i64) -> i64 {
        let id = self.alloc_id();
        self.conversations
            .add(id, player_entity_id, npc_entity_id)
            .await;
        id
    }
}

impl Default for Engagements {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engagement() -> Engagement {
        Engagement::new(1, EngagementType::Battle, vec![10, 20, 30])
    }

    #[test]
    fn new_sets_room_id_to_none() {
        let eng = make_engagement();
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

    fn test_battle_participants() -> (Vec<String>, HashMap<String, Vec<i64>>) {
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);
        participants.insert("enemy".to_string(), vec![2]);
        (
            vec!["player".to_string(), "enemy".to_string()],
            participants,
        )
    }

    #[tokio::test]
    async fn add_battle_and_add_conversation_return_distinct_ids() {
        let engagements = Engagements::new();
        let (factions, participants) = test_battle_participants();
        let id1 = engagements
            .add_battle("room1".to_string(), factions, participants)
            .await;
        let id2 = engagements.add_conversation(3, 4).await;
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn add_battle_sets_room_id_and_type() {
        let engagements = Engagements::new();
        let (factions, participants) = test_battle_participants();
        let id = engagements
            .add_battle("room1".to_string(), factions, participants)
            .await;
        let map = engagements.battles.map.read().await;
        let eng = map.get(&id).unwrap();
        assert_eq!(eng.room_id, Some("room1".to_string()));
        assert_eq!(eng.engagement_type, EngagementType::Battle);
    }

    #[tokio::test]
    async fn find_battle_for_room_returns_id_when_present() {
        let engagements = Engagements::new();
        let (factions, participants) = test_battle_participants();
        let id = engagements
            .add_battle("room1".to_string(), factions, participants)
            .await;
        assert_eq!(engagements.battles.find_for_room("room1").await, Some(id));
    }

    #[tokio::test]
    async fn find_battle_for_room_returns_none_for_different_room() {
        let engagements = Engagements::new();
        let (factions, participants) = test_battle_participants();
        engagements
            .add_battle("room1".to_string(), factions, participants)
            .await;
        assert_eq!(engagements.battles.find_for_room("room2").await, None);
    }

    #[tokio::test]
    async fn find_battle_for_room_returns_none_for_conversation() {
        let engagements = Engagements::new();
        engagements.add_conversation(1, 2).await;
        assert_eq!(engagements.battles.find_for_room("room1").await, None);
    }

    #[tokio::test]
    async fn tick_battles_advances_battle_phase() {
        let engagements = Engagements::new();
        let (factions, participants) = test_battle_participants();
        engagements
            .add_battle("room1".to_string(), factions, participants)
            .await;
        let results = engagements.battles.tick_all(30).await;
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].phase,
            BattlePhase::AnnounceState {
                faction: "player".into()
            }
        );
    }

    #[tokio::test]
    async fn update_battle_participants_removes_dead_and_returns_count() {
        let engagements = Engagements::new();
        let (factions, participants) = test_battle_participants();
        let id = engagements
            .add_battle("room1".to_string(), factions, participants)
            .await;
        let surviving = engagements.battles.update_participants(id, &[1]).await;
        assert_eq!(surviving, 1);
    }

    #[tokio::test]
    async fn conclude_battle_sets_concluded_phase() {
        let engagements = Engagements::new();
        let (factions, participants) = test_battle_participants();
        let id = engagements
            .add_battle("room1".to_string(), factions, participants)
            .await;
        engagements.battles.conclude(id).await;
        let map = engagements.battles.map.read().await;
        let eng = map.get(&id).unwrap();
        assert_eq!(
            eng.battle.as_ref().unwrap().turn_phase,
            BattlePhase::Concluded
        );
    }

    #[tokio::test]
    async fn add_conversation_sets_type_and_entity_ids() {
        let engagements = Engagements::new();
        let id = engagements.add_conversation(10, 20).await;
        let map = engagements.conversations.map.read().await;
        let eng = map.get(&id).unwrap();
        assert_eq!(eng.engagement_type, EngagementType::Conversation);
        assert!(eng.entity_ids.contains(&10));
        assert!(eng.entity_ids.contains(&20));
    }

    #[tokio::test]
    async fn battles_remove_drops_engagement() {
        let engagements = Engagements::new();
        let (factions, participants) = test_battle_participants();
        let id = engagements
            .add_battle("room1".to_string(), factions, participants)
            .await;
        engagements.battles.remove(id).await;
        let map = engagements.battles.map.read().await;
        assert!(!map.contains_key(&id));
    }

    #[tokio::test]
    async fn conversations_remove_drops_engagement() {
        let engagements = Engagements::new();
        let id = engagements.add_conversation(10, 20).await;
        engagements.conversations.remove(id).await;
        let map = engagements.conversations.map.read().await;
        assert!(!map.contains_key(&id));
    }
}
