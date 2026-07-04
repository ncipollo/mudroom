use std::collections::HashMap;

use crate::game::component::{Ability, Attribute};
use crate::game::engagement::{Engagement, EngagementType, Engagements, TurnOrder};

use super::{BattleAiContext, BattleEngagement, BattlePhase, BattleTick};

impl Engagement {
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
}

impl Engagements {
    pub async fn add_battle(
        &self,
        room_id: String,
        factions: Vec<String>,
        participants: HashMap<String, Vec<i64>>,
    ) -> i64 {
        let id = self.alloc_id();
        let engagement = Engagement::new_battle(id, room_id, factions, participants);
        self.engagements_by_id.write().await.insert(id, engagement);
        id
    }

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

    pub async fn conclude_battle(&self, engagement_id: i64) {
        let mut map = self.engagements_by_id.write().await;
        if let Some(engagement) = map.get_mut(&engagement_id)
            && let Some(battle) = &mut engagement.battle
        {
            battle.turn_phase = BattlePhase::Concluded;
        }
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

    #[test]
    fn new_battle_sets_room_id_and_type() {
        let (factions, participants) = test_participants();
        let eng = Engagement::new_battle(5, "room1".to_string(), factions, participants);
        assert_eq!(eng.room_id, Some("room1".to_string()));
        assert_eq!(eng.engagement_type, EngagementType::Battle);
        let mut ids = eng.entity_ids.clone();
        ids.sort();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn new_battle_initializes_battle_engagement() {
        let (factions, participants) = test_participants();
        let eng = Engagement::new_battle(5, "room1".to_string(), factions, participants);
        assert!(eng.battle.is_some());
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
        assert_eq!(surviving, 1);
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
}
