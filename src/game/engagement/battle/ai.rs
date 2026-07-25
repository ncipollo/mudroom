pub mod decision;
pub mod simple_random;

use std::collections::HashMap;
use std::sync::Arc;

use crate::game::GameState;
use crate::game::config::BattleAiType;
use crate::game::entity::Entity;

use super::BattlePhase;

use decision::AiDecision;

pub struct BattleAiContext {
    pub engagement_id: i64,
    pub phase: BattlePhase,
    pub planning_ids: Vec<i64>,
    pub responding_ids: Vec<i64>,
}

pub async fn run_battle_ai(game_state: &Arc<GameState>) {
    let contexts = game_state.engagements.battles.get_ai_contexts().await;
    for ctx in contexts {
        run_ai_for_context(game_state, &ctx).await;
    }
}

async fn run_ai_for_context(game_state: &Arc<GameState>, ctx: &BattleAiContext) {
    let decisions = {
        let entities = game_state.active_entities.read().await;
        collect_decisions(ctx, &entities)
    };
    for decision in decisions {
        match decision {
            AiDecision::Action(boxed) => {
                let (entity_id, ability, target_id, attrs) = *boxed;
                game_state
                    .engagements
                    .battles
                    .queue_ability(entity_id, ability, target_id, &attrs)
                    .await;
            }
            AiDecision::Skip(entity_id) => {
                game_state.engagements.battles.skip_phase(entity_id).await;
            }
        }
    }
}

fn collect_decisions(ctx: &BattleAiContext, entities: &HashMap<i64, Entity>) -> Vec<AiDecision> {
    match &ctx.phase {
        BattlePhase::DeclareAttacks { .. } => ctx
            .planning_ids
            .iter()
            .filter_map(|&id| route_attack(entities.get(&id)?, &ctx.responding_ids))
            .collect(),
        BattlePhase::DeclareDefense { .. } => ctx
            .responding_ids
            .iter()
            .filter_map(|&id| route_defend(entities.get(&id)?))
            .collect(),
        _ => vec![],
    }
}

fn route_attack(entity: &Entity, targets: &[i64]) -> Option<AiDecision> {
    match entity.battle_ai.ai_type {
        BattleAiType::None => None,
        BattleAiType::SimpleRandom => Some(simple_random::plan_attack(entity, targets)),
    }
}

fn route_defend(entity: &Entity) -> Option<AiDecision> {
    match entity.battle_ai.ai_type {
        BattleAiType::None => None,
        BattleAiType::SimpleRandom => Some(simple_random::plan_defend(entity)),
    }
}

pub fn pick_random<T>(items: &[T]) -> Option<&T> {
    if items.is_empty() {
        None
    } else {
        Some(&items[fastrand::usize(..items.len())])
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::game::component::effect::{
        Effect, EffectDescription, EffectScope, EffectType, TriggerInfo,
    };
    use crate::game::component::{Ability, AbilityRole, AbilityTargetType, Location};
    use crate::game::config::BattleAiConfig;
    use crate::game::engagement::EngagementType;
    use crate::game::entity::{Entity, EntityType};

    use super::*;

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    fn make_ability(id: &str, role: AbilityRole, target: AbilityTargetType) -> Ability {
        Ability {
            id: id.to_string(),
            name: id.to_string(),
            description: None,
            effects: vec![Effect {
                name: "dmg".to_string(),
                effect_type: EffectType::AttributeUpdate {
                    attribute_id: "hp".to_string(),
                    value: -5,
                },
                trigger_info: TriggerInfo::Once,
                description: EffectDescription::default(),
                scope: EffectScope::default(),
            }],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![],
            modifiers: vec![],
            role,
            targets: vec![target],
            action_text: None,
        }
    }

    fn make_entity(id: i64, ai_type: BattleAiType) -> Entity {
        let mut e = Entity::new(id, EntityType::Enemy, test_location());
        e.battle_ai = BattleAiConfig { ai_type };
        e.innate_abilities = vec![
            make_ability("attack", AbilityRole::Attack, AbilityTargetType::Opponent),
            make_ability("defend", AbilityRole::Defend, AbilityTargetType::SelfTarget),
        ];
        e
    }

    fn make_entities(list: Vec<Entity>) -> HashMap<i64, Entity> {
        list.into_iter().map(|e| (e.id, e)).collect()
    }

    fn planning_ctx(planning_ids: Vec<i64>, responding_ids: Vec<i64>) -> BattleAiContext {
        BattleAiContext {
            engagement_id: 1,
            phase: BattlePhase::DeclareAttacks {
                faction: "enemy".to_string(),
            },
            planning_ids,
            responding_ids,
        }
    }

    fn response_ctx(planning_ids: Vec<i64>, responding_ids: Vec<i64>) -> BattleAiContext {
        BattleAiContext {
            engagement_id: 1,
            phase: BattlePhase::DeclareDefense {
                faction: "enemy".to_string(),
            },
            planning_ids,
            responding_ids,
        }
    }

    #[test]
    fn collect_decisions_planning_simple_random_queues_attack() {
        let entities = make_entities(vec![make_entity(1, BattleAiType::SimpleRandom)]);
        let ctx = planning_ctx(vec![1], vec![2]);
        let decisions = collect_decisions(&ctx, &entities);
        assert_eq!(decisions.len(), 1);
        assert!(matches!(&decisions[0], AiDecision::Action(b) if b.0 == 1 && b.2 == 2));
    }

    #[test]
    fn collect_decisions_response_simple_random_queues_defend() {
        let entities = make_entities(vec![make_entity(2, BattleAiType::SimpleRandom)]);
        let ctx = response_ctx(vec![1], vec![2]);
        let decisions = collect_decisions(&ctx, &entities);
        assert_eq!(decisions.len(), 1);
        assert!(matches!(&decisions[0], AiDecision::Action(b) if b.0 == 2 && b.2 == 2));
    }

    #[test]
    fn collect_decisions_none_type_is_ignored() {
        let entities = make_entities(vec![make_entity(1, BattleAiType::None)]);
        let ctx = planning_ctx(vec![1], vec![2]);
        assert!(collect_decisions(&ctx, &entities).is_empty());
    }

    #[test]
    fn collect_decisions_simple_random_no_attack_abilities_skips() {
        let mut entity = Entity::new(1, EntityType::Enemy, test_location());
        entity.battle_ai = BattleAiConfig {
            ai_type: BattleAiType::SimpleRandom,
        };
        let entities = make_entities(vec![entity]);
        let ctx = planning_ctx(vec![1], vec![2]);
        let decisions = collect_decisions(&ctx, &entities);
        assert_eq!(decisions.len(), 1);
        assert!(matches!(decisions[0], AiDecision::Skip(1)));
    }

    #[test]
    fn collect_decisions_simple_random_no_defend_abilities_skips() {
        let mut entity = Entity::new(2, EntityType::Enemy, test_location());
        entity.battle_ai = BattleAiConfig {
            ai_type: BattleAiType::SimpleRandom,
        };
        let entities = make_entities(vec![entity]);
        let ctx = response_ctx(vec![1], vec![2]);
        let decisions = collect_decisions(&ctx, &entities);
        assert_eq!(decisions.len(), 1);
        assert!(matches!(decisions[0], AiDecision::Skip(2)));
    }

    #[test]
    fn collect_decisions_non_declare_phase_returns_empty() {
        let entities = make_entities(vec![make_entity(1, BattleAiType::SimpleRandom)]);
        let ctx = BattleAiContext {
            engagement_id: 1,
            phase: BattlePhase::ResolveAbilities,
            planning_ids: vec![1],
            responding_ids: vec![2],
        };
        assert!(collect_decisions(&ctx, &entities).is_empty());
    }

    #[test]
    fn pick_random_returns_none_for_empty() {
        let items: Vec<i32> = vec![];
        assert!(pick_random(&items).is_none());
    }

    #[test]
    fn pick_random_returns_item_for_single_element() {
        let items = vec![42_i32];
        assert_eq!(pick_random(&items), Some(&42));
    }
}
