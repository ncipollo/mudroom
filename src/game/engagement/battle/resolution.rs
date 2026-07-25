use std::collections::HashMap;
use std::sync::Arc;

use crate::game::GameState;
use crate::game::component::effect::{Effect, EffectType, TriggerInfo};
use crate::game::config::AttributeConfig;
use crate::game::engagement::TurnOrder;
use crate::game::entity::Entity;
use crate::game::narration::{TextResolver, VariableMap, effect_text};

use super::{BattleMessage, QueuedAbility};

#[derive(Default)]
struct ResolutionContext {
    once_shields: Vec<Effect>,
}

pub fn resolve_effects(
    target_id: i64,
    mut effects: Vec<Effect>,
    entities: &mut HashMap<i64, Entity>,
) -> Vec<BattleMessage> {
    let Some(entity) = entities.get_mut(&target_id) else {
        return vec![];
    };
    effects.sort_by_key(|e| e.effect_type.resolution_order());
    let mut context = ResolutionContext::default();
    for effect in &effects {
        resolve_effect(effect, entity, &mut context);
    }
    vec![]
}

/// Applies triggered over-time ("innate") effects for the given participants and sweeps any
/// `OverTime` effects whose `end` has passed. Only call this when the current battle tick has
/// just completed the `ApplyEffects` phase — `turn_count` stays fixed for a whole faction turn,
/// so calling this on every raw tick would re-fire matching effects once per raw tick instead of
/// once per turn.
pub(super) async fn apply_innate_effects(
    game_state: &Arc<GameState>,
    participant_ids: &[i64],
    turn_count: u64,
    messages: &mut Vec<BattleMessage>,
) {
    let mut entities = game_state.active_entities.write().await;
    for &entity_id in participant_ids {
        let Some(entity) = entities.get(&entity_id) else {
            continue;
        };
        let entity_name = entity.name.clone();
        let triggered: Vec<Effect> = entity
            .active_effects
            .iter()
            .filter(|e| is_over_time_triggered(&e.trigger_info, turn_count))
            .cloned()
            .collect();
        let expired_names: Vec<String> = entity
            .active_effects
            .iter()
            .filter(|e| is_over_time_expired(&e.trigger_info, turn_count))
            .map(|e| e.name.clone())
            .collect();

        if !triggered.is_empty() {
            let applied = resolve_effects(entity_id, triggered, &mut entities);
            messages.extend(applied);
        }

        if !expired_names.is_empty()
            && let Some(entity) = entities.get_mut(&entity_id)
        {
            entity
                .active_effects
                .retain(|e| !is_over_time_expired(&e.trigger_info, turn_count));
        }
        for effect_name in expired_names {
            messages.push(BattleMessage::EffectExpired {
                entity_name: entity_name.clone(),
                effect_name,
            });
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

fn is_over_time_expired(trigger: &TriggerInfo, turn_count: u64) -> bool {
    matches!(trigger, TriggerInfo::OverTime { end: Some(end), .. } if turn_count >= *end)
}

/// Resolves all queued ability casts from a `ResolveAbilities` phase completion: orders casters
/// by speed, applies each ability's effects to its target, and emits cast messages. Only call
/// this when the current battle tick has just completed the `ResolveAbilities` phase.
pub(super) async fn apply_battle_effects(
    game_state: &Arc<GameState>,
    resolution_queue: Vec<QueuedAbility>,
    entity_names: &HashMap<i64, String>,
    messages: &mut Vec<BattleMessage>,
) {
    let mut entities = game_state.active_entities.write().await;
    let speed_sorted_casters = speed_sort_casters(
        &resolution_queue
            .iter()
            .map(|qa| qa.caster_id)
            .collect::<Vec<_>>(),
        &entities,
        &game_state.attribute_config,
    );

    let mut target_effects = HashMap::new();
    let mut by_caster: HashMap<i64, Vec<QueuedAbility>> = HashMap::new();
    for qa in resolution_queue {
        by_caster.entry(qa.caster_id).or_default().push(qa);
    }

    for &caster_id in &speed_sorted_casters {
        for qa in by_caster.remove(&caster_id).unwrap_or_default() {
            messages.extend(ability_cast_messages(&qa, entity_names));
            target_effects
                .entry(qa.target_id)
                .or_insert_with(Vec::new)
                .extend(qa.ability.effects.clone());
        }
    }
    for abilities in by_caster.into_values() {
        for qa in abilities {
            messages.extend(ability_cast_messages(&qa, entity_names));
            target_effects
                .entry(qa.target_id)
                .or_insert_with(Vec::new)
                .extend(qa.ability.effects.clone());
        }
    }

    for (target_id, effects) in target_effects {
        let applied = resolve_effects(target_id, effects, &mut entities);
        messages.extend(applied);
    }
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

fn ability_cast_messages(
    qa: &QueuedAbility,
    entity_names: &HashMap<i64, String>,
) -> Vec<BattleMessage> {
    let caster_name = entity_names
        .get(&qa.caster_id)
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string());
    let target_name = entity_names
        .get(&qa.target_id)
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string());

    let effect_lines: Vec<BattleMessage> = qa
        .ability
        .effects
        .iter()
        .map(effect_text)
        .filter(|s| !s.is_empty())
        .map(BattleMessage::EffectText)
        .collect();

    let cast_message = if let Some(action_text) = &qa.ability.action_text {
        let vars = VariableMap::new()
            .insert("entity", &caster_name)
            .insert("target", &target_name);
        BattleMessage::Meta(TextResolver::resolve(action_text, &vars))
    } else {
        BattleMessage::AbilityCast {
            caster_name,
            target_name,
            ability_name: qa.ability.name.clone(),
        }
    };

    std::iter::once(cast_message).chain(effect_lines).collect()
}

fn resolve_effect(effect: &Effect, entity: &mut Entity, context: &mut ResolutionContext) {
    match &effect.effect_type {
        EffectType::AttributeShield { .. } => apply_attribute_shield(effect, entity, context),
        EffectType::AttributeUpdate { .. } => apply_attribute_update(effect, entity, context),
        EffectType::EntitySpawn { .. } => apply_entity_spawn(effect, entity, context),
    }
}

fn apply_attribute_shield(effect: &Effect, entity: &mut Entity, context: &mut ResolutionContext) {
    match effect.trigger_info {
        TriggerInfo::Once => context.once_shields.push(effect.clone()),
        TriggerInfo::OverTime { .. } => entity.active_effects.push(effect.clone()),
    }
}

fn apply_attribute_update(effect: &Effect, entity: &mut Entity, context: &mut ResolutionContext) {
    let EffectType::AttributeUpdate {
        attribute_id,
        value,
    } = &effect.effect_type
    else {
        return;
    };
    if effect.trigger_info != TriggerInfo::Once {
        return;
    }
    let adjusted = absorb_with_shields(&mut context.once_shields, attribute_id, *value);
    if let Some(attr) = entity.attributes.get_mut(attribute_id) {
        attr.current_value = (attr.current_value + adjusted)
            .max(attr.min_value)
            .min(attr.max_value);
    }
}

fn apply_entity_spawn(_effect: &Effect, _entity: &mut Entity, _context: &mut ResolutionContext) {}

fn absorb_with_shields(once_shields: &mut Vec<Effect>, attribute_id: &str, value: i64) -> i64 {
    if value >= 0 {
        return value;
    }
    let mut remaining = value;
    let mut consumed = Vec::new();
    for (i, shield) in once_shields.iter_mut().enumerate() {
        if remaining == 0 {
            break;
        }
        if let EffectType::AttributeShield {
            attribute_id: shield_attr,
            absorb_amount,
        } = &mut shield.effect_type
        {
            if shield_attr != attribute_id {
                continue;
            }
            let absorbed = (*absorb_amount).min(remaining.unsigned_abs() as i64);
            remaining = (remaining + absorbed).min(0);
            *absorb_amount = (*absorb_amount - absorbed).max(0);
            if *absorb_amount == 0 {
                consumed.push(i);
            }
        }
    }
    for i in consumed.into_iter().rev() {
        once_shields.remove(i);
    }
    remaining
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::Ability;
    use crate::game::component::AbilityRole;
    use crate::game::component::Attribute;
    use crate::game::component::Location;
    use crate::game::component::effect::{
        Effect, EffectDescription, EffectScope, EffectType, TriggerInfo,
    };
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

    fn damage_effect(value: i64) -> Effect {
        Effect {
            name: "damage".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
            scope: EffectScope::default(),
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
            scope: EffectScope::default(),
        }
    }

    fn attack_ability(damage: i64) -> Ability {
        Ability {
            id: "attack".to_string(),
            name: "Attack".to_string(),
            description: None,
            effects: vec![damage_effect(damage)],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![],
            modifiers: vec![],
            role: AbilityRole::Attack,
            targets: vec![],
            action_text: None,
        }
    }

    fn single_entity(hp: i64) -> HashMap<i64, Entity> {
        let mut entities = HashMap::new();
        let mut entity = Entity::new(1, EntityType::Player, test_location());
        entity.attributes.insert("hp".to_string(), hp_attribute(hp));
        entities.insert(1, entity);
        entities
    }

    #[test]
    fn shield_absorbs_partial_damage_and_is_consumed() {
        let mut entities = single_entity(100);
        // Shield and damage come through the same resolution pass
        resolve_effects(
            1,
            vec![shield_effect(5, TriggerInfo::Once), damage_effect(-10)],
            &mut entities,
        );

        let entity = entities.get(&1).unwrap();
        assert_eq!(entity.attributes["hp"].current_value, 95);
        assert!(entity.active_effects.is_empty());
    }

    #[test]
    fn over_time_shield_is_stubbed_and_not_applied() {
        let mut entities = single_entity(100);
        let over_time_shield = shield_effect(
            5,
            TriggerInfo::OverTime {
                start: 0,
                end: None,
                rate: 1,
            },
        );

        resolve_effects(1, vec![over_time_shield, damage_effect(-10)], &mut entities);

        let entity = entities.get(&1).unwrap();
        // OverTime shield pushed to active_effects but not applied — full damage lands
        assert_eq!(entity.attributes["hp"].current_value, 90);
        assert_eq!(entity.active_effects.len(), 1);
    }

    #[test]
    fn shield_does_not_affect_positive_attribute_updates() {
        let mut entities = single_entity(50);
        let effects = vec![shield_effect(5, TriggerInfo::Once), damage_effect(10)];

        resolve_effects(1, effects, &mut entities);

        let entity = entities.get(&1).unwrap();
        assert_eq!(entity.attributes["hp"].current_value, 60);
        // Shield is still present since heals don't trigger it
        // (shield was in same-pass effects, not active_effects — it's simply not consumed)
        assert!(entity.active_effects.is_empty());
    }

    #[test]
    fn defend_and_attack_in_same_pass_shield_intercepts() {
        let mut entities = single_entity(100);

        // Simulate: defend (shield 5) + attack (-10) queued for same target in same resolution pass
        let effects = vec![shield_effect(5, TriggerInfo::Once), damage_effect(-10)];

        resolve_effects(1, effects, &mut entities);

        let entity = entities.get(&1).unwrap();
        // Shield absorbed 5, 5 damage gets through
        assert_eq!(entity.attributes["hp"].current_value, 95);
        assert!(entity.active_effects.is_empty());
    }

    #[test]
    fn shield_fully_absorbs_attack() {
        let mut entities = single_entity(100);
        let effects = vec![shield_effect(20, TriggerInfo::Once), damage_effect(-10)];

        resolve_effects(1, effects, &mut entities);

        let entity = entities.get(&1).unwrap();
        assert_eq!(entity.attributes["hp"].current_value, 100);
        assert!(entity.active_effects.is_empty());
    }

    #[test]
    fn shield_depletes_across_multiple_hits_until_exhausted() {
        let mut entities = single_entity(100);
        let effects = vec![
            shield_effect(8, TriggerInfo::Once),
            damage_effect(-5),
            damage_effect(-5),
        ];

        resolve_effects(1, effects, &mut entities);

        let entity = entities.get(&1).unwrap();
        // Shield absorbs 5 from first hit (3 remaining), then 3 from second (exhausted),
        // leaving 2 damage through
        assert_eq!(entity.attributes["hp"].current_value, 98);
        assert!(entity.active_effects.is_empty());
    }

    #[test]
    fn absorb_clamps_to_zero_not_positive() {
        let mut entities = single_entity(100);
        let effects = vec![shield_effect(20, TriggerInfo::Once), damage_effect(-10)];

        resolve_effects(1, effects, &mut entities);

        let entity = entities.get(&1).unwrap();
        assert_eq!(entity.attributes["hp"].current_value, 100);
    }

    #[test]
    fn unknown_target_is_noop() {
        let mut entities: HashMap<i64, Entity> = HashMap::new();
        let messages = resolve_effects(99, vec![damage_effect(-10)], &mut entities);
        assert!(messages.is_empty());
    }

    #[test]
    fn applying_shield_effect_directly_adds_to_active_effects_via_over_time() {
        // Verifies OverTime shields are stored for future use
        let mut entities = single_entity(100);
        let over_time_shield = shield_effect(
            5,
            TriggerInfo::OverTime {
                start: 0,
                end: None,
                rate: 1,
            },
        );

        resolve_effects(1, vec![over_time_shield], &mut entities);

        let entity = entities.get(&1).unwrap();
        assert_eq!(entity.active_effects.len(), 1);
        assert_eq!(entity.attributes["hp"].current_value, 100);
    }

    #[test]
    fn attack_ability_effects_resolve_correctly() {
        let mut entities = single_entity(100);
        resolve_effects(1, attack_ability(-10).effects, &mut entities);
        assert_eq!(entities[&1].attributes["hp"].current_value, 90);
    }

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

    fn hp_effect(value: i64, text: Option<&str>) -> Effect {
        Effect {
            name: "damage".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription {
                text: text.map(|s| s.to_string()),
                ..Default::default()
            },
            scope: EffectScope::default(),
        }
    }

    #[test]
    fn no_action_text_no_effects_returns_ability_cast() {
        let qa = QueuedAbility {
            caster_id: 1,
            ability: make_ability(None),
            target_id: 2,
        };
        let msgs = ability_cast_messages(&qa, &make_names());
        assert_eq!(
            msgs,
            vec![BattleMessage::AbilityCast {
                caster_name: "Alice".to_string(),
                target_name: "Bob".to_string(),
                ability_name: "Test Strike".to_string(),
            }]
        );
    }

    #[test]
    fn no_action_text_with_effect_appends_indented_effect_line() {
        let mut ability = make_ability(None);
        ability.effects = vec![hp_effect(-5, Some("Defend for {{abs_value}}"))];
        let qa = QueuedAbility {
            caster_id: 1,
            ability,
            target_id: 2,
        };
        let msgs = ability_cast_messages(&qa, &make_names());
        assert_eq!(
            msgs,
            vec![
                BattleMessage::AbilityCast {
                    caster_name: "Alice".to_string(),
                    target_name: "Bob".to_string(),
                    ability_name: "Test Strike".to_string(),
                },
                BattleMessage::EffectText("Defend for 5".to_string()),
            ]
        );
    }

    #[test]
    fn with_action_text_no_effects_returns_single_meta() {
        let qa = QueuedAbility {
            caster_id: 1,
            ability: make_ability(Some("{{entity}} strikes {{target}}!")),
            target_id: 2,
        };
        let msgs = ability_cast_messages(&qa, &make_names());
        assert_eq!(
            msgs,
            vec![BattleMessage::Meta("Alice strikes Bob!".to_string())]
        );
    }

    #[test]
    fn with_action_text_and_hp_effect_emits_separate_indented_line() {
        let mut ability = make_ability(Some("{{entity}} hits {{target}}"));
        ability.effects = vec![hp_effect(-10, None)];
        let qa = QueuedAbility {
            caster_id: 1,
            ability,
            target_id: 2,
        };
        let msgs = ability_cast_messages(&qa, &make_names());
        assert_eq!(
            msgs,
            vec![
                BattleMessage::Meta("Alice hits Bob".to_string()),
                BattleMessage::EffectText("deals 10 damage".to_string()),
            ]
        );
    }

    #[test]
    fn with_action_text_and_custom_effect_text_uses_resolved_text() {
        let mut ability = make_ability(Some("{{entity}} swings ax at {{target}}"));
        ability.effects = vec![hp_effect(-15, Some("Axe chops for {{abs_value}}"))];
        let qa = QueuedAbility {
            caster_id: 1,
            ability,
            target_id: 2,
        };
        let msgs = ability_cast_messages(&qa, &make_names());
        assert_eq!(
            msgs,
            vec![
                BattleMessage::Meta("Alice swings ax at Bob".to_string()),
                BattleMessage::EffectText("Axe chops for 15".to_string()),
            ]
        );
    }

    fn over_time_effect(value: i64, start: u64, end: Option<u64>, rate: u64) -> Effect {
        Effect {
            name: "poison".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value,
            },
            trigger_info: TriggerInfo::OverTime { start, end, rate },
            description: EffectDescription::default(),
            scope: EffectScope::Battle,
        }
    }

    /// `AttributeShield` effects are re-resolved (and re-pushed onto `active_effects`) every time
    /// they're passed through `resolve_effects`, unlike `AttributeUpdate` — which only applies
    /// for `Once`-triggered effects (see `resolve_effects` docs). That makes shield effects a
    /// useful observable proxy for "was this over-time effect actually resolved this call."
    fn over_time_shield_effect(
        absorb_amount: i64,
        start: u64,
        end: Option<u64>,
        rate: u64,
    ) -> Effect {
        Effect {
            name: "warding".to_string(),
            effect_type: EffectType::AttributeShield {
                attribute_id: "hp".to_string(),
                absorb_amount,
            },
            trigger_info: TriggerInfo::OverTime { start, end, rate },
            description: EffectDescription::default(),
            scope: EffectScope::Battle,
        }
    }

    async fn game_state_with_entity(entity: Entity) -> std::sync::Arc<crate::game::GameState> {
        let game_state = std::sync::Arc::new(crate::game::GameState::load(None).unwrap());
        game_state
            .active_entities
            .write()
            .await
            .insert(entity.id, entity);
        game_state
    }

    #[tokio::test]
    async fn apply_innate_effects_resolves_triggered_over_time_effect() {
        let mut entity = Entity::new(1, EntityType::Player, test_location());
        entity
            .attributes
            .insert("hp".to_string(), hp_attribute(100));
        entity
            .active_effects
            .push(over_time_shield_effect(5, 0, None, 1));
        let game_state = game_state_with_entity(entity).await;

        let mut messages = Vec::new();
        apply_innate_effects(&game_state, &[1], 0, &mut messages).await;

        let entities = game_state.active_entities.read().await;
        // The triggered effect is resolved and re-pushed onto active_effects, proving it
        // actually ran (rather than being silently skipped by the trigger-gate check).
        assert_eq!(entities[&1].active_effects.len(), 2);
    }

    #[tokio::test]
    async fn apply_innate_effects_removes_expired_over_time_effect_and_emits_message() {
        let mut entity = Entity::new(1, EntityType::Player, test_location());
        entity.name = "Goblin".to_string();
        entity
            .attributes
            .insert("hp".to_string(), hp_attribute(100));
        entity
            .active_effects
            .push(over_time_effect(-1, 0, Some(3), 1));
        let game_state = game_state_with_entity(entity).await;

        let mut messages = Vec::new();
        apply_innate_effects(&game_state, &[1], 3, &mut messages).await;

        let entities = game_state.active_entities.read().await;
        assert!(entities[&1].active_effects.is_empty());
        assert!(messages.contains(&BattleMessage::EffectExpired {
            entity_name: "Goblin".to_string(),
            effect_name: "poison".to_string(),
        }));
    }

    #[tokio::test]
    async fn apply_innate_effects_still_triggers_on_the_tick_before_expiry() {
        let mut entity = Entity::new(1, EntityType::Player, test_location());
        entity
            .attributes
            .insert("hp".to_string(), hp_attribute(100));
        entity
            .active_effects
            .push(over_time_shield_effect(5, 0, Some(3), 1));
        let game_state = game_state_with_entity(entity).await;

        let mut messages = Vec::new();
        apply_innate_effects(&game_state, &[1], 2, &mut messages).await;

        let entities = game_state.active_entities.read().await;
        assert_eq!(entities[&1].active_effects.len(), 2);
        assert!(messages.is_empty());
    }
}
