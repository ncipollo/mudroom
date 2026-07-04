use std::collections::HashMap;

use tokio::sync::RwLock;
use tracing;

use crate::game::engagement::{Engagement, EngagementType, ResolvedAction, TurnAction};

use super::factory;

pub struct Conversations {
    pub(in crate::game::engagement) map: RwLock<HashMap<i64, Engagement>>,
}

impl Conversations {
    pub fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::new()),
        }
    }

    pub async fn add(&self, id: i64, player_entity_id: i64, npc_entity_id: i64) {
        let engagement = factory::new_conversation(id, player_entity_id, npc_entity_id);
        self.map.write().await.insert(id, engagement);
    }

    pub async fn remove(&self, engagement_id: i64) {
        self.map.write().await.remove(&engagement_id);
    }

    pub async fn find_for_entity(&self, entity_id: i64) -> Option<(i64, Vec<i64>)> {
        let map = self.map.read().await;
        map.values()
            .find(|e| {
                e.engagement_type == EngagementType::Conversation
                    && e.entity_ids.contains(&entity_id)
            })
            .map(|e| (e.id, e.entity_ids.clone()))
    }

    pub async fn is_entity_in(&self, entity_id: i64) -> bool {
        let map = self.map.read().await;
        map.values().any(|e| {
            e.engagement_type == EngagementType::Conversation && e.entity_ids.contains(&entity_id)
        })
    }

    pub async fn submit_action_for_entity(&self, entity_id: i64, action: TurnAction) -> bool {
        let mut map = self.map.write().await;
        for engagement in map.values_mut() {
            if engagement.entity_ids.contains(&entity_id) {
                return engagement.submit_action(entity_id, action);
            }
        }
        false
    }

    pub async fn process_tick(&self, max_engage_ticks: u64) -> Vec<ResolvedAction> {
        let mut resolved = Vec::new();
        let mut map = self.map.write().await;
        for engagement in map.values_mut() {
            if let Some(action) = tick_engagement(engagement, max_engage_ticks) {
                resolved.push(action);
            }
        }
        resolved
    }
}

impl Default for Conversations {
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

    async fn make_conversations(player_id: i64, npc_id: i64) -> Conversations {
        let conversations = Conversations::new();
        conversations.add(1, player_id, npc_id).await;
        conversations
    }

    #[tokio::test]
    async fn find_for_entity_returns_engagement_when_present() {
        let conversations = make_conversations(10, 20).await;
        let result = conversations.find_for_entity(10).await;
        assert!(result.is_some());
        let (id, ids) = result.unwrap();
        assert_eq!(id, 1);
        assert!(ids.contains(&10));
        assert!(ids.contains(&20));
    }

    #[tokio::test]
    async fn find_for_entity_returns_none_when_absent() {
        let conversations = make_conversations(10, 20).await;
        assert!(conversations.find_for_entity(99).await.is_none());
    }

    #[tokio::test]
    async fn is_entity_in_returns_true_when_present() {
        let conversations = make_conversations(10, 20).await;
        assert!(conversations.is_entity_in(10).await);
        assert!(conversations.is_entity_in(20).await);
    }

    #[tokio::test]
    async fn is_entity_in_returns_false_when_absent() {
        let conversations = make_conversations(10, 20).await;
        assert!(!conversations.is_entity_in(99).await);
    }

    #[tokio::test]
    async fn submit_action_accepted_for_entity_in_conversation() {
        let conversations = make_conversations(10, 20).await;
        let action = TurnAction::SendMessage {
            content: "hi".to_string(),
        };
        assert!(conversations.submit_action_for_entity(10, action).await);
    }

    #[tokio::test]
    async fn submit_action_rejected_for_unknown_entity() {
        let conversations = make_conversations(10, 20).await;
        let action = TurnAction::SendMessage {
            content: "hi".to_string(),
        };
        assert!(!conversations.submit_action_for_entity(99, action).await);
    }

    #[tokio::test]
    async fn process_tick_advances_turn_after_action_submitted() {
        let conversations = make_conversations(10, 20).await;
        conversations
            .submit_action_for_entity(
                10,
                TurnAction::Respond {
                    content: "ok".to_string(),
                },
            )
            .await;
        conversations.process_tick(30).await;
        let map = conversations.map.read().await;
        let eng = map.values().next().unwrap();
        assert_eq!(eng.current_entity(), Some(10));
    }

    #[tokio::test]
    async fn process_tick_increments_ticks_when_no_action() {
        let conversations = make_conversations(10, 20).await;
        conversations.process_tick(30).await;
        let map = conversations.map.read().await;
        let eng = map.values().next().unwrap();
        assert_eq!(eng.ticks_on_current_turn, 1);
    }

    #[tokio::test]
    async fn process_tick_advances_on_timeout() {
        let conversations = make_conversations(10, 20).await;
        for _ in 0..=3 {
            conversations.process_tick(3).await;
        }
        let map = conversations.map.read().await;
        let eng = map.values().next().unwrap();
        assert_eq!(eng.ticks_on_current_turn, 0);
    }

    #[tokio::test]
    async fn remove_drops_engagement() {
        let conversations = make_conversations(10, 20).await;
        conversations.remove(1).await;
        let map = conversations.map.read().await;
        assert!(!map.contains_key(&1));
    }
}
