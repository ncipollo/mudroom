use std::collections::HashMap;

use tokio::sync::RwLock;

use crate::game::component::effect::Effect;
use crate::game::component::{Ability, Attribute, ResetCondition};
use crate::game::config::AttributeConfig;
use crate::game::engagement::{Engagement, EngagementType};
use crate::game::entity::Entity;

use super::{BattleAiContext, BattleMessage, BattlePhase, BattleTick, factory, resolution};

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

    pub async fn tick_all(
        &self,
        max_engage_ticks: u64,
        entities: &HashMap<i64, Entity>,
        attribute_config: &AttributeConfig,
    ) -> Vec<BattleTick> {
        let mut map = self.map.write().await;
        let mut results = Vec::new();
        for engagement in map.values_mut() {
            if let Some(battle) = &mut engagement.battle {
                results.push(battle.tick(
                    engagement.id,
                    max_engage_ticks,
                    entities,
                    attribute_config,
                ));
            }
        }
        results
    }

    /// Resolves effects for a target within a specific battle engagement, routing attribute
    /// updates through that engagement's pending attribute working copy rather than the
    /// entity's actual state.
    pub async fn resolve_battle_effects(
        &self,
        engagement_id: i64,
        target_id: i64,
        effects: Vec<Effect>,
        entities: &mut HashMap<i64, Entity>,
    ) -> Vec<BattleMessage> {
        let mut map = self.map.write().await;
        let Some(engagement) = map.get_mut(&engagement_id) else {
            return vec![];
        };
        let Some(battle) = &mut engagement.battle else {
            return vec![];
        };
        resolution::resolve_effects(
            target_id,
            effects,
            entities,
            &mut battle.pending_entity_attributes,
        )
    }

    /// Copies every `Never`-reset-condition attribute from the engagement's pending working
    /// copy back into the actual entities. Called once, at the end of the Resolution phase,
    /// after this round's queued effects have already been applied to pending.
    pub async fn flush_never_attributes(
        &self,
        engagement_id: i64,
        attribute_config: &AttributeConfig,
        entities: &mut HashMap<i64, Entity>,
    ) {
        let map = self.map.read().await;
        let Some(engagement) = map.get(&engagement_id) else {
            return;
        };
        let Some(battle) = &engagement.battle else {
            return;
        };
        let never_ids: Vec<&str> = attribute_config
            .attributes
            .iter()
            .filter(|def| def.reset_condition == ResetCondition::Never)
            .map(|def| def.id.as_str())
            .collect();
        for (entity_id, pending_attrs) in &battle.pending_entity_attributes {
            let Some(entity) = entities.get_mut(entity_id) else {
                continue;
            };
            for &attr_id in &never_ids {
                if let Some(attr) = pending_attrs.get(attr_id) {
                    entity.attributes.insert(attr_id.to_string(), attr.clone());
                }
            }
        }
    }

    /// Returns a clone of the engagement's pending attribute working copy (empty if the
    /// engagement or its battle isn't found), used to prefer in-turn pending values over actual
    /// entity state when making battle decisions (e.g. turn order).
    pub async fn pending_attributes(
        &self,
        engagement_id: i64,
    ) -> HashMap<i64, HashMap<String, Attribute>> {
        let map = self.map.read().await;
        map.get(&engagement_id)
            .and_then(|engagement| engagement.battle.as_ref())
            .map(|battle| battle.pending_entity_attributes.clone())
            .unwrap_or_default()
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
        let results = battles
            .tick_all(30, &HashMap::new(), &AttributeConfig::default_config())
            .await;
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

    fn never_hp_config() -> AttributeConfig {
        use crate::game::component::{AttributeCategory, AttributeDefinition};

        AttributeConfig {
            attributes: vec![AttributeDefinition {
                id: "hp".to_string(),
                title: "Hit Points".to_string(),
                description: String::new(),
                min_value: 0,
                max_value: 100,
                attribute_type: crate::game::component::AttributeType::HP,
                attribute_category: AttributeCategory::Life,
                reset_condition: ResetCondition::Never,
            }],
        }
    }

    fn entity_with_hp(id: i64, hp: i64) -> Entity {
        use crate::game::component::Location;
        use crate::game::entity::EntityType;

        let loc = Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        };
        let mut entity = Entity::new(id, EntityType::Player, loc);
        entity.attributes.insert(
            "hp".to_string(),
            Attribute::new("hp".to_string(), 0, 100, hp),
        );
        entity
    }

    #[tokio::test]
    async fn flush_never_attributes_copies_pending_hp_to_actual() {
        let battles = make_battles().await;
        let mut entities = HashMap::new();
        entities.insert(1, entity_with_hp(1, 100));

        {
            let mut map = battles.map.write().await;
            let battle = map.get_mut(&1).unwrap().battle.as_mut().unwrap();
            battle.pending_entity_attributes.insert(
                1,
                HashMap::from([(
                    "hp".to_string(),
                    Attribute::new("hp".to_string(), 0, 100, 42),
                )]),
            );
        }

        battles
            .flush_never_attributes(1, &never_hp_config(), &mut entities)
            .await;

        assert_eq!(entities[&1].attributes["hp"].current_value, 42);
    }

    #[tokio::test]
    async fn flush_never_attributes_leaves_pending_intact() {
        let battles = make_battles().await;
        let mut entities = HashMap::new();
        entities.insert(1, entity_with_hp(1, 100));

        {
            let mut map = battles.map.write().await;
            let battle = map.get_mut(&1).unwrap().battle.as_mut().unwrap();
            battle.pending_entity_attributes.insert(
                1,
                HashMap::from([(
                    "hp".to_string(),
                    Attribute::new("hp".to_string(), 0, 100, 42),
                )]),
            );
        }

        battles
            .flush_never_attributes(1, &never_hp_config(), &mut entities)
            .await;

        let map = battles.map.read().await;
        let battle = map[&1].battle.as_ref().unwrap();
        assert_eq!(battle.pending_entity_attributes[&1]["hp"].current_value, 42);
    }

    #[tokio::test]
    async fn resolve_battle_effects_writes_pending_not_actual() {
        use crate::game::component::effect::{
            Effect, EffectDescription, EffectScope, EffectType, TriggerInfo,
        };

        let battles = make_battles().await;
        let mut entities = HashMap::new();
        entities.insert(1, entity_with_hp(1, 100));
        let effects = vec![Effect {
            name: "damage".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -10,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
            scope: EffectScope::default(),
        }];

        battles
            .resolve_battle_effects(1, 1, effects, &mut entities)
            .await;

        assert_eq!(entities[&1].attributes["hp"].current_value, 100);
        let map = battles.map.read().await;
        let battle = map[&1].battle.as_ref().unwrap();
        assert_eq!(battle.pending_entity_attributes[&1]["hp"].current_value, 90);
    }
}
