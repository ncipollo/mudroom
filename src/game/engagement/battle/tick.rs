use std::collections::HashMap;
use std::sync::Arc;

use crate::game::component::{Ability, AttributeType, EffectType, TriggerInfo};
use crate::game::config::AttributeConfig;
use crate::game::engagement::TurnOrder;
use crate::game::entity::Entity;
use crate::game::messaging::{BattleParticipantInfo, BattleUpdateMessage};
use crate::game::{GameState, messaging};

use super::loot;
use super::{
    BattleMessage, BattlePhase, BattleTick, QueuedAbility, entity_innate_battle_abilities,
};

pub async fn handle_tick(game_state: &Arc<GameState>, result: BattleTick, max_engage_ticks: u64) {
    let engagement_id = result.engagement_id;
    let all_ids = result.all_participant_ids.clone();

    let (innate_jobs, entity_names, speed_sorted_casters) =
        collect_entity_data(game_state, &result).await;

    let mut cast_messages = Vec::new();
    apply_battle_effects(
        game_state,
        &innate_jobs,
        result.resolution_queue,
        &speed_sorted_casters,
        &entity_names,
        &mut cast_messages,
    )
    .await;

    let dead_ids = detect_dead_entities(game_state, &all_ids, &game_state.attribute_config).await;

    let death_messages: Vec<BattleMessage> = dead_ids
        .iter()
        .map(|&id| BattleMessage::EntityDied {
            name: entity_names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string()),
        })
        .collect();

    let all_tick_messages: Vec<BattleMessage> = result
        .messages
        .iter()
        .chain(cast_messages.iter())
        .chain(death_messages.iter())
        .cloned()
        .collect();

    let player_ids = find_participant_player_ids(game_state, &all_ids).await;

    let surviving = game_state
        .engagements
        .update_battle_participants(engagement_id, &dead_ids)
        .await;

    let countdown_ticks = max_engage_ticks.saturating_sub(result.ticks_in_phase);
    let params = BattleUpdateParams {
        engagement_id,
        factions: result.factions,
        participants: result.participants,
        phase: result.phase,
        messages: all_tick_messages,
        countdown_ticks,
        max_turn_ticks: max_engage_ticks,
    };
    let update = build_battle_update(game_state, params).await;

    for &pid in &player_ids {
        messaging::battle_update(&game_state.message_tx, pid, update.clone());
    }

    if surviving <= 1 {
        loot::resolve_loot(&all_ids);
        game_state.engagements.conclude_battle(engagement_id).await;
        clear_active_effects(game_state, &all_ids).await;
        for &pid in &player_ids {
            messaging::battle_ended(&game_state.message_tx, pid, engagement_id);
        }
        game_state.engagements.remove(engagement_id).await;
    }
}

async fn collect_entity_data(
    game_state: &Arc<GameState>,
    result: &BattleTick,
) -> (Vec<(i64, Vec<Ability>)>, HashMap<i64, String>, Vec<i64>) {
    let entities = game_state.active_entities.read().await;
    let players = game_state.active_players.read().await;

    let innate_jobs: Vec<(i64, Vec<Ability>)> = result
        .innate_entity_ids
        .iter()
        .filter_map(|&eid| {
            let entity = entities.get(&eid)?;
            let abilities = entity_innate_battle_abilities(entity);
            if abilities.is_empty() {
                None
            } else {
                Some((eid, abilities))
            }
        })
        .collect();

    let entity_names: HashMap<i64, String> = result
        .all_participant_ids
        .iter()
        .map(|&eid| {
            let name = players
                .values()
                .find(|p| p.entity_id == eid)
                .map(|p| p.name.clone())
                .or_else(|| {
                    entities
                        .get(&eid)
                        .and_then(|e| e.config_id.as_deref())
                        .map(str::to_string)
                })
                .unwrap_or_else(|| format!("Entity {eid}"));
            (eid, name)
        })
        .collect();

    let speed_sorted_casters = speed_sort_casters(
        &result
            .resolution_queue
            .iter()
            .map(|qa| qa.caster_id)
            .collect::<Vec<_>>(),
        &entities,
        &game_state.attribute_config,
    );

    (innate_jobs, entity_names, speed_sorted_casters)
}

fn speed_sort_casters(
    caster_ids: &[i64],
    entities: &HashMap<i64, Entity>,
    config: &AttributeConfig,
) -> Vec<i64> {
    let entity_refs: Vec<&Entity> = caster_ids
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .filter_map(|&id| entities.get(&id))
        .collect();
    TurnOrder::new_from_entities(&entity_refs, config)
        .order()
        .to_vec()
}

async fn apply_battle_effects(
    game_state: &Arc<GameState>,
    innate_jobs: &[(i64, Vec<Ability>)],
    resolution_queue: Vec<QueuedAbility>,
    speed_sorted_casters: &[i64],
    entity_names: &HashMap<i64, String>,
    cast_messages: &mut Vec<BattleMessage>,
) {
    let mut entities = game_state.active_entities.write().await;

    for (entity_id, abilities) in innate_jobs {
        for ability in abilities {
            apply_once_effects(ability, *entity_id, &mut entities);
        }
    }

    let mut by_caster: HashMap<i64, Vec<QueuedAbility>> = HashMap::new();
    for qa in resolution_queue {
        by_caster.entry(qa.caster_id).or_default().push(qa);
    }

    for &caster_id in speed_sorted_casters {
        if let Some(abilities) = by_caster.remove(&caster_id) {
            for qa in &abilities {
                apply_once_effects(&qa.ability, qa.target_id, &mut entities);
                cast_messages.push(ability_cast_message(qa, entity_names));
            }
        }
    }
    for abilities in by_caster.into_values() {
        for qa in &abilities {
            apply_once_effects(&qa.ability, qa.target_id, &mut entities);
            cast_messages.push(ability_cast_message(qa, entity_names));
        }
    }
}

fn ability_cast_message(qa: &QueuedAbility, entity_names: &HashMap<i64, String>) -> BattleMessage {
    BattleMessage::AbilityCast {
        caster_name: entity_names
            .get(&qa.caster_id)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string()),
        target_name: entity_names
            .get(&qa.target_id)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string()),
        ability_name: qa.ability.name.clone(),
    }
}

fn absorb_with_shields(entity: &mut Entity, attribute_id: &str, value: i64) -> i64 {
    if value >= 0 {
        return value;
    }
    let mut remaining = value;
    let mut consumed_once = Vec::new();
    for (i, effect) in entity.active_effects.iter().enumerate() {
        if let EffectType::AttributeShield {
            attribute_id: shield_attr,
            absorb_amount,
        } = &effect.effect_type
            && shield_attr == attribute_id
        {
            remaining = (remaining + absorb_amount).min(0);
            if matches!(effect.trigger_info, TriggerInfo::Once) {
                consumed_once.push(i);
            }
        }
    }
    for i in consumed_once.into_iter().rev() {
        entity.active_effects.remove(i);
    }
    remaining
}

fn apply_once_effects(ability: &Ability, target_id: i64, entities: &mut HashMap<i64, Entity>) {
    let Some(entity) = entities.get_mut(&target_id) else {
        return;
    };
    for effect in &ability.effects {
        match (&effect.trigger_info, &effect.effect_type) {
            (
                TriggerInfo::Once,
                EffectType::AttributeUpdate {
                    attribute_id,
                    value,
                },
            ) => {
                let adjusted = absorb_with_shields(entity, attribute_id, *value);
                if let Some(attr) = entity.attributes.get_mut(attribute_id) {
                    attr.current_value = (attr.current_value + adjusted)
                        .max(attr.min_value)
                        .min(attr.max_value);
                }
            }
            (_, EffectType::AttributeShield { .. }) => {
                entity.active_effects.push(effect.clone());
            }
            _ => {}
        }
    }
}

async fn detect_dead_entities(
    game_state: &Arc<GameState>,
    entity_ids: &[i64],
    config: &AttributeConfig,
) -> Vec<i64> {
    let hp_def_ids: Vec<&str> = config
        .attributes
        .iter()
        .filter(|def| matches!(def.attribute_type, AttributeType::HP))
        .map(|def| def.id.as_str())
        .collect();

    let entities = game_state.active_entities.read().await;
    entity_ids
        .iter()
        .filter(|&&id| is_entity_dead(id, &entities, &hp_def_ids))
        .copied()
        .collect()
}

fn is_entity_dead(entity_id: i64, entities: &HashMap<i64, Entity>, hp_def_ids: &[&str]) -> bool {
    let Some(entity) = entities.get(&entity_id) else {
        return false;
    };
    hp_def_ids.iter().any(|&hp_id| {
        entity
            .attributes
            .get(hp_id)
            .is_some_and(|attr| attr.current_value <= attr.min_value)
    })
}

async fn clear_active_effects(game_state: &Arc<GameState>, entity_ids: &[i64]) {
    let mut entities = game_state.active_entities.write().await;
    for &id in entity_ids {
        if let Some(entity) = entities.get_mut(&id) {
            entity.active_effects.clear();
        }
    }
}

async fn find_participant_player_ids(game_state: &Arc<GameState>, entity_ids: &[i64]) -> Vec<i64> {
    let players = game_state.active_players.read().await;
    players
        .values()
        .filter(|p| entity_ids.contains(&p.entity_id))
        .map(|p| p.id)
        .collect()
}

struct BattleUpdateParams {
    engagement_id: i64,
    factions: Vec<String>,
    participants: HashMap<String, Vec<i64>>,
    phase: BattlePhase,
    messages: Vec<BattleMessage>,
    countdown_ticks: u64,
    max_turn_ticks: u64,
}

async fn build_battle_update(
    game_state: &Arc<GameState>,
    params: BattleUpdateParams,
) -> BattleUpdateMessage {
    let entities = game_state.active_entities.read().await;
    let players = game_state.active_players.read().await;
    let hp_attr_id = messaging::hp_attribute_id(&game_state.attribute_config);

    let participant_infos = params
        .participants
        .iter()
        .map(|(faction, ids)| {
            let infos = ids
                .iter()
                .map(|&id| {
                    let entity = entities.get(&id);
                    let name = players
                        .values()
                        .find(|p| p.entity_id == id)
                        .map(|p| p.name.clone())
                        .or_else(|| {
                            entity
                                .and_then(|e| e.config_id.as_deref())
                                .map(str::to_string)
                        })
                        .unwrap_or_else(|| format!("Entity {id}"));
                    let (hp_current, hp_max) = entity
                        .and_then(|e| e.attributes.get(&hp_attr_id))
                        .map(|a| (a.current_value, a.max_value))
                        .unwrap_or((0, 0));
                    BattleParticipantInfo {
                        id,
                        name,
                        hp_current,
                        hp_max,
                    }
                })
                .collect();
            (faction.clone(), infos)
        })
        .collect();

    BattleUpdateMessage {
        engagement_id: params.engagement_id,
        factions: params.factions,
        participants: participant_infos,
        phase: params.phase,
        messages: params.messages,
        countdown_ticks: params.countdown_ticks,
        max_turn_ticks: params.max_turn_ticks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::Ability;
    use crate::game::component::Attribute;
    use crate::game::component::Location;
    use crate::game::component::effect::{Effect, EffectDescription, EffectType, TriggerInfo};
    use crate::game::engagement::EngagementType;
    use crate::game::entity::{Entity, EntityType};

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    fn hp_attribute(current: i64) -> Attribute {
        Attribute::new("hp".to_string(), 0, 100, current)
    }

    fn attack_ability(damage: i64) -> Ability {
        Ability {
            id: "attack".to_string(),
            name: "Attack".to_string(),
            description: None,
            effects: vec![Effect {
                name: "damage".to_string(),
                effect_type: EffectType::AttributeUpdate {
                    attribute_id: "hp".to_string(),
                    value: damage,
                },
                trigger_info: TriggerInfo::Once,
                description: EffectDescription::default(),
            }],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![],
            modifiers: vec![],
        }
    }

    fn shield_effect(absorb_amount: i64, trigger: TriggerInfo) -> Effect {
        Effect {
            name: "damage_reduction".to_string(),
            effect_type: EffectType::AttributeShield {
                attribute_id: "hp".to_string(),
                absorb_amount,
            },
            trigger_info: trigger,
            description: EffectDescription::default(),
        }
    }

    #[test]
    fn shield_absorbs_partial_damage_and_is_consumed() {
        let mut entities: HashMap<i64, Entity> = HashMap::new();
        let mut entity = Entity::new(1, EntityType::Player, test_location());
        entity
            .attributes
            .insert("hp".to_string(), hp_attribute(100));
        entity
            .active_effects
            .push(shield_effect(5, TriggerInfo::Once));
        entities.insert(1, entity);

        apply_once_effects(&attack_ability(-10), 1, &mut entities);

        let entity = entities.get(&1).unwrap();
        assert_eq!(entity.attributes["hp"].current_value, 95);
        assert!(entity.active_effects.is_empty());
    }

    #[test]
    fn over_time_shield_absorbs_and_is_not_consumed() {
        let mut entities: HashMap<i64, Entity> = HashMap::new();
        let mut entity = Entity::new(1, EntityType::Player, test_location());
        entity
            .attributes
            .insert("hp".to_string(), hp_attribute(100));
        entity.active_effects.push(shield_effect(
            5,
            TriggerInfo::OverTime {
                start: 0,
                end: None,
                rate: 1,
            },
        ));
        entities.insert(1, entity);

        apply_once_effects(&attack_ability(-10), 1, &mut entities);

        let entity = entities.get(&1).unwrap();
        assert_eq!(entity.attributes["hp"].current_value, 95);
        assert_eq!(entity.active_effects.len(), 1);
    }

    #[test]
    fn shield_does_not_affect_positive_attribute_updates() {
        let mut entities: HashMap<i64, Entity> = HashMap::new();
        let mut entity = Entity::new(1, EntityType::Player, test_location());
        entity.attributes.insert("hp".to_string(), hp_attribute(50));
        entity
            .active_effects
            .push(shield_effect(5, TriggerInfo::Once));
        entities.insert(1, entity);

        apply_once_effects(&attack_ability(10), 1, &mut entities);

        let entity = entities.get(&1).unwrap();
        assert_eq!(entity.attributes["hp"].current_value, 60);
        assert_eq!(entity.active_effects.len(), 1);
    }

    #[test]
    fn applying_shield_ability_adds_to_active_effects() {
        let mut entities: HashMap<i64, Entity> = HashMap::new();
        let mut entity = Entity::new(1, EntityType::Player, test_location());
        entity
            .attributes
            .insert("hp".to_string(), hp_attribute(100));
        entities.insert(1, entity);

        let defend_ability = Ability {
            id: "defend".to_string(),
            name: "Defend".to_string(),
            description: None,
            effects: vec![shield_effect(5, TriggerInfo::Once)],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![],
            modifiers: vec![],
        };
        apply_once_effects(&defend_ability, 1, &mut entities);

        let entity = entities.get(&1).unwrap();
        assert_eq!(entity.active_effects.len(), 1);
        assert_eq!(entity.attributes["hp"].current_value, 100);
    }

    #[test]
    fn absorb_with_shields_clamps_to_zero_not_positive() {
        let mut entities: HashMap<i64, Entity> = HashMap::new();
        let mut entity = Entity::new(1, EntityType::Player, test_location());
        entity
            .attributes
            .insert("hp".to_string(), hp_attribute(100));
        entity
            .active_effects
            .push(shield_effect(20, TriggerInfo::Once));
        entities.insert(1, entity);

        apply_once_effects(&attack_ability(-10), 1, &mut entities);

        let entity = entities.get(&1).unwrap();
        assert_eq!(entity.attributes["hp"].current_value, 100);
    }
}
