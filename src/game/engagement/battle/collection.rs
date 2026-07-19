use std::collections::HashMap;

use tokio::sync::RwLock;

use crate::game::component::{Ability, Attribute};
use crate::game::engagement::{Engagement, EngagementType};

use super::{BattleAiContext, BattlePhase, BattleTick, factory};

pub struct Battles {
    pub(in crate::game::engagement) map: RwLock<HashMap<i64, Engagement>>,
}

impl Battles {
    pub fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::new()),
        }
    }

    pub async fn add(
        &self,
        id: i64,
        room_id: String,
        factions: Vec<String>,
        participants: HashMap<String, Vec<i64>>,
    ) {
        let engagement = factory::new_battle(id, room_id, factions, participants);
        self.map.write().await.insert(id, engagement);
    }

    pub async fn remove(&self, engagement_id: i64) {
        self.map.write().await.remove(&engagement_id);
    }

    pub async fn find_for_room(&self, room_id: &str) -> Option<i64> {
        let map = self.map.read().await;
        map.values()
            .find(|e| {
                e.engagement_type == EngagementType::Battle && e.room_id.as_deref() == Some(room_id)
            })
            .map(|e| e.id)
    }

    pub async fn tick_all(&self, max_engage_ticks: u64) -> Vec<BattleTick> {
        let mut map = self.map.write().await;
        let mut results = Vec::new();
        for engagement in map.values_mut() {
            if let Some(battle) = &mut engagement.battle {
                results.push(battle.tick(engagement.id, max_engage_ticks));
            }
        }
        results
    }

    pub async fn update_participants(&self, engagement_id: i64, dead_entity_ids: &[i64]) -> usize {
        let mut map = self.map.write().await;
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

    pub async fn queue_ability(
        &self,
        entity_id: i64,
        ability: Ability,
        target_id: i64,
        entity_attrs: &HashMap<String, Attribute>,
    ) -> bool {
        let mut map = self.map.write().await;
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

    pub async fn skip_phase(&self, entity_id: i64) {
        let mut map = self.map.write().await;
        for engagement in map.values_mut() {
            if engagement.engagement_type == EngagementType::Battle
                && engagement.entity_ids.contains(&entity_id)
                && let Some(battle) = &mut engagement.battle
            {
                battle.skip_phase(entity_id);
                return;
            }
        }
    }

    pub async fn get_ai_contexts(&self) -> Vec<BattleAiContext> {
        let map = self.map.read().await;
        map.values()
            .filter_map(|e| {
                let battle = e.battle.as_ref()?;
                match &battle.turn_phase {
                    BattlePhase::Planning { .. } | BattlePhase::Response { .. } => {
                        Some(BattleAiContext {
                            engagement_id: e.id,
                            phase: battle.turn_phase.clone(),
                            planning_ids: battle.unacted_planning_ids(),
                            responding_ids: battle.unacted_responding_ids(),
                        })
                    }
                    _ => None,
                }
            })
            .collect()
    }

    pub async fn conclude(&self, engagement_id: i64) {
        let mut map = self.map.write().await;
        if let Some(engagement) = map.get_mut(&engagement_id)
            && let Some(battle) = &mut engagement.battle
        {
            battle.turn_phase = BattlePhase::Concluded;
        }
    }
}

impl Default for Battles {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::engagement::battle::factory;

    fn test_participants() -> (Vec<String>, HashMap<String, Vec<i64>>) {
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);
        participants.insert("enemy".to_string(), vec![2]);
        (
            vec!["player".to_string(), "enemy".to_string()],
            participants,
        )
    }

    async fn make_battles() -> Battles {
        let battles = Battles::new();
        let (factions, participants) = test_participants();
        battles
            .add(1, "room1".to_string(), factions, participants)
            .await;
        battles
    }

    #[tokio::test]
    async fn find_for_room_returns_id_when_present() {
        let battles = make_battles().await;
        assert_eq!(battles.find_for_room("room1").await, Some(1));
    }

    #[tokio::test]
    async fn find_for_room_returns_none_for_different_room() {
        let battles = make_battles().await;
        assert_eq!(battles.find_for_room("room2").await, None);
    }

    #[tokio::test]
    async fn find_for_room_returns_none_when_empty() {
        let battles = Battles::new();
        assert_eq!(battles.find_for_room("room1").await, None);
    }

    #[tokio::test]
    async fn tick_all_advances_battle_phase() {
        let battles = make_battles().await;
        let results = battles.tick_all(30).await;
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].phase,
            BattlePhase::Planning {
                faction: "player".into()
            }
        );
    }

    #[tokio::test]
    async fn update_participants_removes_dead_and_returns_count() {
        let battles = make_battles().await;
        let surviving = battles.update_participants(1, &[1]).await;
        assert_eq!(surviving, 1);
    }

    #[tokio::test]
    async fn conclude_sets_concluded_phase() {
        let battles = make_battles().await;
        battles.conclude(1).await;
        let map = battles.map.read().await;
        let battle = map[&1].battle.as_ref().unwrap();
        assert_eq!(battle.turn_phase, BattlePhase::Concluded);
    }

    #[tokio::test]
    async fn remove_drops_engagement() {
        let battles = make_battles().await;
        battles.remove(1).await;
        let map = battles.map.read().await;
        assert!(!map.contains_key(&1));
    }

    #[tokio::test]
    async fn add_creates_engagement_with_correct_room() {
        let battles = make_battles().await;
        let map = battles.map.read().await;
        let eng = map.get(&1).unwrap();
        assert_eq!(eng.room_id, Some("room1".to_string()));
        assert_eq!(eng.engagement_type, EngagementType::Battle);
    }
}
