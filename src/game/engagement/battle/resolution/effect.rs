use std::collections::HashMap;

use crate::game::component::effect::{Effect, EffectType, TriggerInfo};
use crate::game::config::AttributeConfig;
use crate::game::engagement::battle::BattleMessage;
use crate::game::entity::character::Character;

#[derive(Default)]
struct ResolutionContext {
    once_shields: Vec<Effect>,
}

/// Resolves a batch of effects against a single target. Effects are sorted by
/// `resolution_order()` first so `Once` attribute shields register before `Once` attribute
/// updates land in the same pass, letting shields absorb same-pass damage.
pub(super) fn resolve_effects(
    target_id: i64,
    mut effects: Vec<Effect>,
    entities: &mut HashMap<i64, Character>,
    attribute_config: &AttributeConfig,
) -> Vec<BattleMessage> {
    let Some(character) = entities.get_mut(&target_id) else {
        return vec![];
    };
    effects.sort_by_key(|e| e.effect_type.resolution_order());
    let mut context = ResolutionContext::default();
    for effect in &effects {
        resolve_effect(effect, character, &mut context, attribute_config);
    }
    vec![]
}

fn resolve_effect(
    effect: &Effect,
    character: &mut Character,
    context: &mut ResolutionContext,
    attribute_config: &AttributeConfig,
) {
    match &effect.effect_type {
        EffectType::AttributeShield { .. } => apply_attribute_shield(effect, character, context),
        EffectType::AttributeBuff { .. } => apply_attribute_buff(effect, character),
        EffectType::AttributeUpdate { .. } => {
            apply_attribute_update(effect, character, context, attribute_config)
        }
        EffectType::EntitySpawn { .. } => apply_entity_spawn(effect, character, context),
    }
}

/// A buff is inherently durational (unlike a `Once` attribute update, which permanently mutates
/// `current_value`), so it always lands in `active_effects` regardless of its trigger kind.
/// Callers read the effective value through `Character::effective_attribute` rather than here.
fn apply_attribute_buff(effect: &Effect, character: &mut Character) {
    character.active_effects.push(effect.clone());
}

fn apply_attribute_shield(
    effect: &Effect,
    character: &mut Character,
    context: &mut ResolutionContext,
) {
    match effect.trigger_info {
        TriggerInfo::Once => context.once_shields.push(effect.clone()),
        TriggerInfo::OverTime { .. } => character.active_effects.push(effect.clone()),
    }
}

fn apply_attribute_update(
    effect: &Effect,
    character: &mut Character,
    context: &mut ResolutionContext,
    attribute_config: &AttributeConfig,
) {
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
    // Shields absorb same-pass damage regardless of whether the attribute is updatable, so
    // absorption always runs before the updatable check short-circuits the write.
    let adjusted = absorb_with_shields(&mut context.once_shields, attribute_id, *value);
    if !attribute_config.is_updatable(attribute_id) {
        return;
    }
    if let Some(attr) = character.attributes.get_mut(attribute_id) {
        attr.current_value = (attr.current_value + adjusted)
            .max(attr.min_value)
            .min(attr.max_value);
        tracing::info!(
            entity_id = character.id,
            attribute_id = %attribute_id,
            pending_value = attr.current_value,
            "Pending attribute impacted"
        );
    }
}

fn apply_entity_spawn(_effect: &Effect, _entity: &mut Character, _context: &mut ResolutionContext) {
}

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
mod tests;
