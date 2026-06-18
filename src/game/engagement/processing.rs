use std::collections::HashMap;
use std::sync::Arc;

use crate::game::component::{Ability, AttributeType, EffectType, TriggerInfo};
use crate::game::config::AttributeConfig;
use crate::game::engagement::battle::loot;
use crate::game::engagement::battle::{BattleMessage, BattleTick, entity_innate_battle_abilities};
use crate::game::engagement::{EngagementType, TurnOrder};
use crate::game::entity::Entity;
use crate::game::{GameState, messaging};

use super::conversation;

/// Process all active engagements for the current game tick.
///
/// Each tick, the engagement system:
/// 1. Calculates the maximum number of ticks a single turn may last before it times out,
///    based on `max_engage_ms / tick_rate_ms` from the mud config.
/// 2. Calls [`crate::game::Engagements::process_tick`] to advance every engagement — any
///    engagement whose current entity has submitted an action (or whose turn has timed out)
///    is resolved and returned as a [`crate::game::ResolvedAction`].
/// 3. Dispatches each resolved action to the appropriate handler based on its
///    [`EngagementType`]. Currently only [`EngagementType::Conversation`] is handled.
/// 4. Advances all battle engagements through their phase state machine and applies effects.
pub async fn process(game_state: &Arc<GameState>, _tick: u64) {
    let max_engage_ticks = (game_state.mud_config.game_loop.max_engage_ms
        / game_state.mud_config.game_loop.tick_rate_ms)
        .max(1);

    let resolved = game_state.engagements.process_tick(max_engage_ticks).await;
    for r in &resolved {
        if r.engagement_type == EngagementType::Conversation {
            conversation::handle(game_state, r).await;
        }
    }

    let battle_results = game_state.engagements.tick_battles(max_engage_ticks).await;
    for result in battle_results {
        handle_battle_tick(game_state, result).await;
    }
}

async fn handle_battle_tick(game_state: &Arc<GameState>, result: BattleTick) {
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

    let player_ids = find_participant_player_ids(game_state, &all_ids).await;
    announce(&game_state.message_tx, &player_ids, &result.messages);
    announce(&game_state.message_tx, &player_ids, &cast_messages);
    announce(&game_state.message_tx, &player_ids, &death_messages);

    let surviving = game_state
        .engagements
        .update_battle_participants(engagement_id, &dead_ids)
        .await;

    if surviving <= 1 {
        loot::resolve_loot(&all_ids);
        game_state.engagements.conclude_battle(engagement_id).await;
        for &pid in &player_ids {
            messaging::message(&game_state.message_tx, pid, "The battle has ended.");
        }
        game_state.engagements.remove(engagement_id).await;
    }
}

async fn collect_entity_data(
    game_state: &Arc<GameState>,
    result: &BattleTick,
) -> (Vec<(i64, Vec<Ability>)>, HashMap<i64, String>, Vec<i64>) {
    let entities = game_state.active_entities.read().await;

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
        .filter_map(|&eid| {
            let name = entities
                .get(&eid)?
                .config_id
                .as_deref()
                .unwrap_or("Unknown")
                .to_string();
            Some((eid, name))
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
    resolution_queue: Vec<crate::game::engagement::battle::QueuedAbility>,
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

    let mut by_caster: HashMap<i64, Vec<crate::game::engagement::battle::QueuedAbility>> =
        HashMap::new();
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

fn ability_cast_message(
    qa: &crate::game::engagement::battle::QueuedAbility,
    entity_names: &HashMap<i64, String>,
) -> BattleMessage {
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

fn apply_once_effects(ability: &Ability, target_id: i64, entities: &mut HashMap<i64, Entity>) {
    let Some(entity) = entities.get_mut(&target_id) else {
        return;
    };
    for effect in &ability.effects {
        if let TriggerInfo::Once = effect.trigger_info
            && let EffectType::AttributeUpdate {
                attribute_id,
                value,
            } = &effect.effect_type
            && let Some(attr) = entity.attributes.get_mut(attribute_id)
        {
            attr.current_value = (attr.current_value + value)
                .max(attr.min_value)
                .min(attr.max_value);
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

async fn find_participant_player_ids(game_state: &Arc<GameState>, entity_ids: &[i64]) -> Vec<i64> {
    let players = game_state.active_players.read().await;
    players
        .values()
        .filter(|p| entity_ids.contains(&p.entity_id))
        .map(|p| p.id)
        .collect()
}

fn announce(
    tx: &tokio::sync::broadcast::Sender<crate::game::messaging::PlayerMessage>,
    player_ids: &[i64],
    messages: &[BattleMessage],
) {
    for msg in messages {
        let text = msg.to_string();
        for &pid in player_ids {
            messaging::message(tx, pid, &text);
        }
    }
}
