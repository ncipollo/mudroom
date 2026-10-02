pub mod battle;
pub mod conversation;
pub mod engagement_type;
pub mod processing;
pub mod resolved_action;
pub mod turn_action;
pub mod turn_order;
pub mod turn_state;

pub use battle::{
    BattleAiContext, BattleEngagement, BattleMessage, BattlePhase, BattleTick, Battles,
    QueuedAbility,
};
pub use conversation::Conversations;
pub use engagement_type::EngagementType;
pub use processing::process;
pub use resolved_action::ResolvedAction;
pub use turn_action::TurnAction;
pub use turn_order::TurnOrder;
pub use turn_state::Engagement;

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

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
