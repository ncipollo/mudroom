use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

use tokio::sync::RwLock;
use tracing;

use crate::game::component::{Ability, Attribute};
use crate::game::engagement::Engagement;
use crate::game::engagement::EngagementType;
use crate::game::engagement::ResolvedAction;
use crate::game::engagement::TurnAction;
use crate::game::engagement::battle::{BattleAiContext, BattlePhase, BattleTick};

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

    /// Create and add a new engagement. Returns the new engagement's id.
    pub async fn add(&self, engagement_type: EngagementType, entity_ids: Vec<i64>) -> i64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let engagement = Engagement::new(id, engagement_type, entity_ids);
        self.engagements_by_id.write().await.insert(id, engagement);
        id
    }

    /// Create a faction-aware battle engagement tied to a specific room. Returns the new id.
    pub async fn add_battle(
        &self,
        room_id: String,
        factions: Vec<String>,
        participants: HashMap<String, Vec<i64>>,
    ) -> i64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let engagement = Engagement::new_battle(id, room_id, factions, participants);
        self.engagements_by_id.write().await.insert(id, engagement);
        id
    }

    /// Returns the engagement id of an active Battle in the given room, or `None`.
    pub async fn find_battle_for_room(&self, room_id: &str) -> Option<i64> {
        self.engagements_by_id
            .read()
            .await
            .values()
            .find(|e| {
                e.engagement_type == EngagementType::Battle && e.room_id.as_deref() == Some(room_id)
            })
            .map(|e| e.id)
    }

    /// Create a conversation engagement where only the player takes turns.
    /// Returns the new engagement's id.
    pub async fn add_conversation(&self, player_entity_id: i64, npc_entity_id: i64) -> i64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let engagement = Engagement::new_conversation(id, player_entity_id, npc_entity_id);
        self.engagements_by_id.write().await.insert(id, engagement);
        id
    }

    pub async fn remove(&self, engagement_id: i64) {
        self.engagements_by_id.write().await.remove(&engagement_id);
    }

    /// Returns the engagement id and all participant entity ids for the conversation
    /// containing the given entity, or `None` if none exists.
    pub async fn find_conversation_for_entity(&self, entity_id: i64) -> Option<(i64, Vec<i64>)> {
        self.engagements_by_id
            .read()
            .await
            .values()
            .find(|e| {
                e.engagement_type == EngagementType::Conversation
                    && e.entity_ids.contains(&entity_id)
            })
            .map(|e| (e.id, e.entity_ids.clone()))
    }

    /// Returns true if the given entity is currently part of a Conversation engagement.
    pub async fn is_entity_in_conversation(&self, entity_id: i64) -> bool {
        self.engagements_by_id.read().await.values().any(|e| {
            e.engagement_type == EngagementType::Conversation && e.entity_ids.contains(&entity_id)
        })
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

    /// Advance all battle engagements by one tick. Returns the tick results for each battle.
    pub async fn tick_battles(&self, max_engage_ticks: u64) -> Vec<BattleTick> {
        let mut results = Vec::new();
        let mut map = self.engagements_by_id.write().await;
        for engagement in map.values_mut() {
            if let Some(battle) = &mut engagement.battle {
                results.push(battle.tick(engagement.id, max_engage_ticks));
            }
        }
        results
    }

    /// Remove dead entities from a battle's participant list. Returns the surviving faction count.
    pub async fn update_battle_participants(
        &self,
        engagement_id: i64,
        dead_entity_ids: &[i64],
    ) -> usize {
        let mut map = self.engagements_by_id.write().await;
        let Some(engagement) = map.get_mut(&engagement_id) else {
            return 0;
        };
        let Some(battle) = &mut engagement.battle else {
            return 0;
        };
        for &dead_id in dead_entity_ids {
            battle.remove_entity(dead_id);
            engagement.entity_ids.retain(|&id| id != dead_id);
        }
        battle.surviving_faction_count()
    }

    /// Queue an ability in the entity's active battle engagement.
    /// Returns false if the entity is not in a battle or lacks resources.
    pub async fn queue_battle_ability(
        &self,
        entity_id: i64,
        ability: Ability,
        target_id: i64,
        entity_attrs: &HashMap<String, Attribute>,
    ) -> bool {
        let mut map = self.engagements_by_id.write().await;
        for engagement in map.values_mut() {
            if engagement.engagement_type == EngagementType::Battle
                && engagement.entity_ids.contains(&entity_id)
                && let Some(battle) = &mut engagement.battle
            {
                return battle.queue_ability(entity_id, ability, target_id, entity_attrs);
            }
        }
        false
    }

    /// Return AI context for every battle currently in Planning or Response phase.
    pub async fn get_battle_ai_contexts(&self) -> Vec<BattleAiContext> {
        let map = self.engagements_by_id.read().await;
        map.values()
            .filter_map(|e| {
                let battle = e.battle.as_ref()?;
                match &battle.turn_phase {
                    BattlePhase::Planning { .. } | BattlePhase::Response { .. } => {
                        Some(BattleAiContext {
                            engagement_id: e.id,
                            phase: battle.turn_phase.clone(),
                            planning_ids: battle.planning_ids(),
                            responding_ids: battle.responding_ids(),
                        })
                    }
                    _ => None,
                }
            })
            .collect()
    }

    /// Mark a battle engagement as concluded.
    pub async fn conclude_battle(&self, engagement_id: i64) {
        let mut map = self.engagements_by_id.write().await;
        if let Some(engagement) = map.get_mut(&engagement_id)
            && let Some(battle) = &mut engagement.battle
        {
            battle.turn_phase = BattlePhase::Concluded;
        }
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

    fn test_participants() -> (Vec<String>, HashMap<String, Vec<i64>>) {
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);
        participants.insert("enemy".to_string(), vec![2]);
        (
            vec!["player".to_string(), "enemy".to_string()],
            participants,
        )
    }

    #[tokio::test]
    async fn add_battle_sets_room_id() {
        let engagements = Engagements::new();
        let (factions, participants) = test_participants();
        let id = engagements
            .add_battle("room1".to_string(), factions, participants)
            .await;
        let map = engagements.engagements_by_id.read().await;
        let eng = map.get(&id).unwrap();
        assert_eq!(eng.room_id, Some("room1".to_string()));
        assert_eq!(eng.engagement_type, EngagementType::Battle);
    }

    #[tokio::test]
    async fn find_battle_for_room_returns_id_when_present() {
        let engagements = Engagements::new();
        let (factions, participants) = test_participants();
        let id = engagements
            .add_battle("room1".to_string(), factions, participants)
            .await;
        assert_eq!(engagements.find_battle_for_room("room1").await, Some(id));
    }

    #[tokio::test]
    async fn find_battle_for_room_returns_none_for_different_room() {
        let engagements = Engagements::new();
        let (factions, participants) = test_participants();
        engagements
            .add_battle("room1".to_string(), factions, participants)
            .await;
        assert_eq!(engagements.find_battle_for_room("room2").await, None);
    }

    #[tokio::test]
    async fn find_battle_for_room_returns_none_for_conversation() {
        let engagements = Engagements::new();
        engagements.add_conversation(1, 2).await;
        assert_eq!(engagements.find_battle_for_room("room1").await, None);
    }

    #[tokio::test]
    async fn tick_battles_advances_battle_phase() {
        let engagements = Engagements::new();
        let (factions, participants) = test_participants();
        engagements
            .add_battle("room1".to_string(), factions, participants)
            .await;
        let results = engagements.tick_battles(30).await;
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].phase,
            crate::game::engagement::battle::BattlePhase::Planning {
                faction: "player".into()
            }
        );
    }

    #[tokio::test]
    async fn update_battle_participants_removes_dead_and_returns_count() {
        let engagements = Engagements::new();
        let (factions, participants) = test_participants();
        let id = engagements
            .add_battle("room1".to_string(), factions, participants)
            .await;
        let surviving = engagements.update_battle_participants(id, &[1]).await;
        assert_eq!(surviving, 1); // only enemy remains
    }

    #[tokio::test]
    async fn conclude_battle_sets_concluded_phase() {
        let engagements = Engagements::new();
        let (factions, participants) = test_participants();
        let id = engagements
            .add_battle("room1".to_string(), factions, participants)
            .await;
        engagements.conclude_battle(id).await;
        let map = engagements.engagements_by_id.read().await;
        let eng = map.get(&id).unwrap();
        assert_eq!(
            eng.battle.as_ref().unwrap().turn_phase,
            crate::game::engagement::battle::BattlePhase::Concluded
        );
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
        // Entity 20 is not the current turn but can still pre-submit an action
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
        // Simulate timeout by processing enough ticks
        for _ in 0..=3 {
            engagements.process_tick(3).await;
        }
        let map = engagements.engagements_by_id.read().await;
        let eng = map.values().next().unwrap();
        assert_eq!(eng.current_entity(), Some(20));
    }
}
