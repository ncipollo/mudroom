use std::collections::HashMap;
use std::sync::Arc;

use crate::game::component::AttributeType;
use crate::game::component::effect::{EffectScope, TriggerInfo};
use crate::game::config::AttributeConfig;
use crate::game::engagement::TurnOrder;
use crate::game::entity::Entity;
use crate::game::messaging::{BattleParticipantInfo, BattleUpdateMessage};
use crate::game::narration::{TextResolver, VariableMap};
use crate::game::{GameState, messaging};

use super::resolution;
use super::{
    BattleMessage, BattlePhase, BattleTick, QueuedAbility, entity_innate_battle_abilities,
};
use super::{loot, timer};

struct BattleTickOutcome {
    engagement_id: i64,
    all_participant_ids: Vec<i64>,
    dead_entity_ids: Vec<i64>,
    player_ids: Vec<i64>,
}

/// Advances all active battle engagements one tick and handles the full lifecycle:
/// phase state machine → effect resolution → dead-entity removal → engagement conclusion.
/// This is the single entry point for battle processing; no battle-specific logic escapes
/// into the engagement orchestration layer.
pub async fn process_ticks(game_state: &Arc<GameState>, max_engage_ticks: u64) {
    let battle_results = game_state
        .engagements
        .battles
        .tick_all(max_engage_ticks)
        .await;
    for result in battle_results {
        let outcome = handle_tick(game_state, result, max_engage_ticks).await;
        let surviving = game_state
            .engagements
            .battles
            .update_participants(outcome.engagement_id, &outcome.dead_entity_ids)
            .await;
        if surviving <= 1 {
            handle_battle_ended(game_state, &outcome).await;
            game_state
                .engagements
                .battles
                .conclude(outcome.engagement_id)
                .await;
            game_state
                .engagements
                .battles
                .remove(outcome.engagement_id)
                .await;
        }
    }
}

/// Resolves effects from a completed battle tick: applies innate and queued ability effects,
/// detects entity deaths, and broadcasts the battle state update to all player participants.
async fn handle_tick(
    game_state: &Arc<GameState>,
    result: BattleTick,
    max_engage_ticks: u64,
) -> BattleTickOutcome {
    let engagement_id = result.engagement_id;
    let all_ids = result.all_participant_ids.clone();

    let (entity_names, speed_sorted_casters) = collect_entity_data(game_state, &result).await;

    let mut cast_messages = Vec::new();
    apply_innate_effects(
        game_state,
        &result.all_participant_ids,
        result.turn_count,
        &mut cast_messages,
    )
    .await;
    apply_battle_effects(
        game_state,
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

    let pending_attack_messages: Vec<BattleMessage> = result
        .pending_actions
        .iter()
        .map(|qa| pending_attack_message(qa, &entity_names))
        .collect();

    let all_tick_messages: Vec<BattleMessage> = result
        .messages
        .iter()
        .chain(pending_attack_messages.iter())
        .chain(cast_messages.iter())
        .chain(death_messages.iter())
        .cloned()
        .collect();

    let player_pairs = find_participant_player_ids(game_state, &all_ids).await;

    let countdown_ticks = max_engage_ticks.saturating_sub(result.ticks_in_phase);
    let tick_rate_ms = game_state.mud_config.game_loop.tick_rate_ms;
    let countdown_secs = timer::ticks_to_secs(countdown_ticks, tick_rate_ms);
    let max_turn_secs = timer::ticks_to_secs(max_engage_ticks, tick_rate_ms);
    let params = BattleUpdateParams {
        engagement_id,
        factions: result.factions,
        participants: result.participants,
        phase: result.phase,
        messages: all_tick_messages,
        countdown_secs,
        max_turn_secs,
    };
    let update = build_battle_update(game_state, params).await;

    {
        let entities = game_state.active_entities.read().await;
        for &(pid, entity_id) in &player_pairs {
            let available_abilities = entities
                .get(&entity_id)
                .map(entity_innate_battle_abilities)
                .unwrap_or_default();
            let mut player_update = update.clone();
            player_update.available_abilities = available_abilities;
            messaging::battle_update(&game_state.message_tx, pid, player_update);
        }
    }

    BattleTickOutcome {
        engagement_id,
        all_participant_ids: all_ids,
        dead_entity_ids: dead_ids,
        player_ids: player_pairs.into_iter().map(|(pid, _)| pid).collect(),
    }
}

async fn handle_battle_ended(game_state: &Arc<GameState>, outcome: &BattleTickOutcome) {
    loot::resolve_loot(&outcome.all_participant_ids);
    clear_active_effects(game_state, &outcome.all_participant_ids).await;
    for &pid in &outcome.player_ids {
        messaging::battle_ended(&game_state.message_tx, pid, outcome.engagement_id);
    }
}

async fn collect_entity_data(
    game_state: &Arc<GameState>,
    result: &BattleTick,
) -> (HashMap<i64, String>, Vec<i64>) {
    let entities = game_state.active_entities.read().await;

    let entity_names: HashMap<i64, String> = result
        .all_participant_ids
        .iter()
        .map(|&eid| {
            let name = entities
                .get(&eid)
                .map(|e| e.name.clone())
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

    (entity_names, speed_sorted_casters)
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

async fn apply_innate_effects(
    game_state: &Arc<GameState>,
    participant_ids: &[i64],
    turn_count: u64,
    cast_messages: &mut Vec<BattleMessage>,
) {
    let mut entities = game_state.active_entities.write().await;
    for &entity_id in participant_ids {
        let Some(entity) = entities.get(&entity_id) else {
            continue;
        };
        let triggered: Vec<_> = entity
            .active_effects
            .iter()
            .filter(|e| is_over_time_triggered(&e.trigger_info, turn_count))
            .cloned()
            .collect();
        if !triggered.is_empty() {
            let messages = resolution::resolve_effects(entity_id, triggered, &mut entities);
            cast_messages.extend(messages);
        }
    }
}

fn is_over_time_triggered(trigger: &TriggerInfo, turn_count: u64) -> bool {
    match trigger {
        TriggerInfo::OverTime { start, end, rate } => {
            turn_count >= *start
                && end.is_none_or(|e| turn_count < e)
                && (turn_count - start).is_multiple_of(*rate)
        }
        TriggerInfo::Once => false,
    }
}

async fn apply_battle_effects(
    game_state: &Arc<GameState>,
    resolution_queue: Vec<QueuedAbility>,
    speed_sorted_casters: &[i64],
    entity_names: &HashMap<i64, String>,
    cast_messages: &mut Vec<BattleMessage>,
) {
    let mut entities = game_state.active_entities.write().await;
    let mut target_effects = HashMap::new();

    let mut by_caster: HashMap<i64, Vec<QueuedAbility>> = HashMap::new();
    for qa in resolution_queue {
        by_caster.entry(qa.caster_id).or_default().push(qa);
    }

    for &caster_id in speed_sorted_casters {
        for qa in by_caster.remove(&caster_id).unwrap_or_default() {
            cast_messages.push(ability_cast_message(&qa, entity_names));
            target_effects
                .entry(qa.target_id)
                .or_insert_with(Vec::new)
                .extend(qa.ability.effects.clone());
        }
    }
    for abilities in by_caster.into_values() {
        for qa in abilities {
            cast_messages.push(ability_cast_message(&qa, entity_names));
            target_effects
                .entry(qa.target_id)
                .or_insert_with(Vec::new)
                .extend(qa.ability.effects.clone());
        }
    }

    for (target_id, effects) in target_effects {
        resolution::resolve_effects(target_id, effects, &mut entities);
    }
}

fn ability_cast_message(qa: &QueuedAbility, entity_names: &HashMap<i64, String>) -> BattleMessage {
    let caster_name = entity_names
        .get(&qa.caster_id)
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string());
    let target_name = entity_names
        .get(&qa.target_id)
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string());

    if let Some(action_text) = &qa.ability.action_text {
        let vars = VariableMap::new()
            .insert("entity", &caster_name)
            .insert("target", &target_name);
        return BattleMessage::Meta(TextResolver::resolve(action_text, &vars));
    }

    BattleMessage::AbilityCast {
        caster_name,
        target_name,
        ability_name: qa.ability.name.clone(),
    }
}

fn pending_attack_message(
    qa: &QueuedAbility,
    entity_names: &HashMap<i64, String>,
) -> BattleMessage {
    BattleMessage::PendingAttack {
        caster_name: entity_names
            .get(&qa.caster_id)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string()),
        ability_name: qa.ability.name.clone(),
        target_name: entity_names
            .get(&qa.target_id)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string()),
        target_id: qa.target_id,
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
            entity
                .active_effects
                .retain(|e| e.scope != EffectScope::Battle);
        }
    }
}

async fn find_participant_player_ids(
    game_state: &Arc<GameState>,
    entity_ids: &[i64],
) -> Vec<(i64, i64)> {
    let players = game_state.active_players.read().await;
    players
        .values()
        .filter(|p| entity_ids.contains(&p.entity_id))
        .map(|p| (p.id, p.entity_id))
        .collect()
}

struct BattleUpdateParams {
    engagement_id: i64,
    factions: Vec<String>,
    participants: HashMap<String, Vec<i64>>,
    phase: BattlePhase,
    messages: Vec<BattleMessage>,
    countdown_secs: u64,
    max_turn_secs: u64,
}

async fn build_battle_update(
    game_state: &Arc<GameState>,
    params: BattleUpdateParams,
) -> BattleUpdateMessage {
    let entities = game_state.active_entities.read().await;
    let hp_attr_id = messaging::hp_attribute_id(&game_state.attribute_config);

    let participant_infos = params
        .participants
        .iter()
        .map(|(faction, ids)| {
            let infos = ids
                .iter()
                .map(|&id| {
                    let entity = entities.get(&id);
                    let name = entity
                        .map(|e| e.name.clone())
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
        countdown_secs: params.countdown_secs,
        max_turn_secs: params.max_turn_secs,
        available_abilities: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::{Ability, AbilityRole};
    use crate::game::engagement::EngagementType;

    fn make_ability(action_text: Option<&str>) -> Ability {
        Ability {
            id: "test".to_string(),
            name: "Test Strike".to_string(),
            description: None,
            effects: vec![],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![],
            modifiers: vec![],
            role: AbilityRole::Attack,
            targets: vec![],
            action_text: action_text.map(|s| s.to_string()),
        }
    }

    fn make_names() -> HashMap<i64, String> {
        let mut m = HashMap::new();
        m.insert(1, "Alice".to_string());
        m.insert(2, "Bob".to_string());
        m
    }

    #[test]
    fn no_action_text_returns_ability_cast() {
        let qa = QueuedAbility {
            caster_id: 1,
            ability: make_ability(None),
            target_id: 2,
        };
        let msg = ability_cast_message(&qa, &make_names());
        assert_eq!(
            msg,
            BattleMessage::AbilityCast {
                caster_name: "Alice".to_string(),
                target_name: "Bob".to_string(),
                ability_name: "Test Strike".to_string(),
            }
        );
    }

    #[test]
    fn with_action_text_returns_meta_with_resolved_text() {
        let qa = QueuedAbility {
            caster_id: 1,
            ability: make_ability(Some("{{entity}} strikes {{target}}!")),
            target_id: 2,
        };
        let msg = ability_cast_message(&qa, &make_names());
        assert_eq!(msg, BattleMessage::Meta("Alice strikes Bob!".to_string()));
    }

    #[test]
    fn with_action_text_effect_stub_left_as_literal() {
        let qa = QueuedAbility {
            caster_id: 1,
            ability: make_ability(Some("{{entity}} hits {{target}} for {{effect}}")),
            target_id: 2,
        };
        let msg = ability_cast_message(&qa, &make_names());
        assert_eq!(
            msg,
            BattleMessage::Meta("Alice hits Bob for {{effect}}".to_string())
        );
    }
}
