pub mod battle;
pub mod conversation;
pub mod engagement_type;
pub mod processing;
pub mod resolved_action;
pub mod turn_action;
pub mod turn_order;

pub use battle::{BattleEngagement, BattleMessage, BattlePhase, QueuedAbility};
pub use engagement_type::EngagementType;
pub use processing::process;
pub use resolved_action::ResolvedAction;
pub use turn_action::TurnAction;
pub use turn_order::TurnOrder;

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

use tokio::sync::RwLock;
use tracing;

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

pub struct Engagements {
    pub(in crate::game::engagement) engagements_by_id: RwLock<HashMap<i64, Engagement>>,
    next_id: AtomicI64,
}

impl Engagements {
    pub fn new() -> Self {
        Self {
            engagements_by_id: RwLock::new(HashMap::new()),
            next_id: AtomicI64::new(1),
        }
    }

    pub(in crate::game::engagement) fn alloc_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Create and add a new engagement. Returns the new engagement's id.
    pub async fn add(&self, engagement_type: EngagementType, entity_ids: Vec<i64>) -> i64 {
        let id = self.alloc_id();
        let engagement = Engagement::new(id, engagement_type, entity_ids);
        self.engagements_by_id.write().await.insert(id, engagement);
        id
    }

    pub async fn remove(&self, engagement_id: i64) {
        self.engagements_by_id.write().await.remove(&engagement_id);
    }

    /// Find the engagement containing the given entity and submit a turn action.
    /// Entities may submit actions off-turn; they are stored per-entity and resolved in order.
    /// Returns true if the entity is part of an engagement.
    pub async fn submit_action_for_entity(&self, entity_id: i64, action: TurnAction) -> bool {
        let mut map = self.engagements_by_id.write().await;
        for engagement in map.values_mut() {
            if engagement.entity_ids.contains(&entity_id) {
                return engagement.submit_action(entity_id, action);
            }
        }
        false
    }

    /// Process one game tick for all engagements. Resolves or times out the current turn
    /// for each engagement where applicable. Returns the list of resolved actions.
    pub async fn process_tick(&self, max_engage_ticks: u64) -> Vec<ResolvedAction> {
        let mut resolved = Vec::new();
        let mut map = self.engagements_by_id.write().await;
        for engagement in map.values_mut() {
            if let Some(action) = tick_engagement(engagement, max_engage_ticks) {
                resolved.push(action);
            }
        }
        resolved
    }
}

impl Default for Engagements {
    fn default() -> Self {
        Self::new()
    }
}

fn tick_engagement(engagement: &mut Engagement, max_engage_ticks: u64) -> Option<ResolvedAction> {
    if !engagement.should_advance(max_engage_ticks) {
        engagement.ticks_on_current_turn += 1;
        return None;
    }
    let id = engagement.current_entity()?;
    let action = engagement.pending_actions.get(&id).cloned();
    log_tick_action(engagement.id, id, &action);
    let resolved = build_resolved_action(engagement, id, action);
    engagement.advance_turn();
    Some(resolved)
}

fn log_tick_action(engagement_id: i64, entity_id: i64, action: &Option<TurnAction>) {
    let resolved = action.is_some();
    tracing::debug!(
        "tick engagement={engagement_id} entity={entity_id} resolved={resolved} action={action:?}"
    );
}

fn build_resolved_action(
    engagement: &Engagement,
    entity_id: i64,
    action: Option<TurnAction>,
) -> ResolvedAction {
    ResolvedAction {
        engagement_id: engagement.id,
        engagement_type: engagement.engagement_type.clone(),
        entity_ids: engagement.entity_ids.clone(),
        entity_id,
        action,
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

    #[tokio::test]
    async fn add_returns_sequential_ids() {
        let engagements = Engagements::new();
        let id1 = engagements.add(EngagementType::Battle, vec![1, 2]).await;
        let id2 = engagements
            .add(EngagementType::Conversation, vec![3, 4])
            .await;
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn remove_drops_engagement() {
        let engagements = Engagements::new();
        let id = engagements.add(EngagementType::Battle, vec![1, 2]).await;
        engagements.remove(id).await;
        let map = engagements.engagements_by_id.read().await;
        assert!(!map.contains_key(&id));
    }

    #[tokio::test]
    async fn submit_action_for_current_entity_succeeds() {
        let engagements = Engagements::new();
        engagements.add(EngagementType::Battle, vec![10, 20]).await;
        let accepted = engagements
            .submit_action_for_entity(
                10,
                TurnAction::SendMessage {
                    content: "attack".to_string(),
                },
            )
            .await;
        assert!(accepted);
    }

    #[tokio::test]
    async fn submit_action_for_off_turn_entity_succeeds() {
        let engagements = Engagements::new();
        engagements.add(EngagementType::Battle, vec![10, 20]).await;
        let accepted = engagements
            .submit_action_for_entity(
                20,
                TurnAction::SendMessage {
                    content: "attack".to_string(),
                },
            )
            .await;
        assert!(accepted);
    }

    #[tokio::test]
    async fn submit_action_for_unknown_entity_fails() {
        let engagements = Engagements::new();
        engagements.add(EngagementType::Battle, vec![10, 20]).await;
        let accepted = engagements
            .submit_action_for_entity(
                99,
                TurnAction::SendMessage {
                    content: "attack".to_string(),
                },
            )
            .await;
        assert!(!accepted);
    }

    #[tokio::test]
    async fn process_tick_advances_turn_after_action_submitted() {
        let engagements = Engagements::new();
        engagements.add(EngagementType::Battle, vec![10, 20]).await;
        engagements
            .submit_action_for_entity(
                10,
                TurnAction::Respond {
                    content: "ok".to_string(),
                },
            )
            .await;
        engagements.process_tick(30).await;
        let map = engagements.engagements_by_id.read().await;
        let eng = map.values().next().unwrap();
        assert_eq!(eng.current_entity(), Some(20));
    }

    #[tokio::test]
    async fn process_tick_increments_ticks_when_no_action() {
        let engagements = Engagements::new();
        engagements.add(EngagementType::Battle, vec![10, 20]).await;
        engagements.process_tick(30).await;
        let map = engagements.engagements_by_id.read().await;
        let eng = map.values().next().unwrap();
        assert_eq!(eng.ticks_on_current_turn, 1);
        assert_eq!(eng.current_entity(), Some(10));
    }

    #[tokio::test]
    async fn process_tick_advances_on_timeout() {
        let engagements = Engagements::new();
        engagements.add(EngagementType::Battle, vec![10, 20]).await;
        for _ in 0..=3 {
            engagements.process_tick(3).await;
        }
        let map = engagements.engagements_by_id.read().await;
        let eng = map.values().next().unwrap();
        assert_eq!(eng.current_entity(), Some(20));
    }
}
