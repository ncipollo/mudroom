use std::collections::HashMap;

use crate::game::component::effect::{Effect, EffectType, TriggerInfo};
use crate::game::entity::Entity;

use super::BattleMessage;

pub fn resolve_effects(
    target_id: i64,
    effects: Vec<Effect>,
    entities: &mut HashMap<i64, Entity>,
) -> Vec<BattleMessage> {
    let Some(entity) = entities.get_mut(&target_id) else {
        return vec![];
    };

    // Shield effects intercept incoming effects before they reach the entity.
    let (shield_effects, incoming_effects): (Vec<Effect>, Vec<Effect>) = effects
        .into_iter()
        .partition(|e| matches!(e.effect_type, EffectType::AttributeShield { .. }));

    let mut once_shields = collect_shield_effects(shield_effects, entity);

    for effect in &incoming_effects {
        if let (
            TriggerInfo::Once,
            EffectType::AttributeUpdate {
                attribute_id,
                value,
            },
        ) = (&effect.trigger_info, &effect.effect_type)
        {
            apply_attribute_update(entity, attribute_id, *value, &mut once_shields);
        }
    }

    vec![]
}

/// Splits shield effects by trigger: OverTime shields are stubbed into `active_effects` for
/// future handling; Once shields are returned for inline absorption this pass.
fn collect_shield_effects(shield_effects: Vec<Effect>, entity: &mut Entity) -> Vec<Effect> {
    let mut once_shields = Vec::new();
    for shield in shield_effects {
        match shield.trigger_info {
            TriggerInfo::OverTime { .. } => entity.active_effects.push(shield),
            TriggerInfo::Once => once_shields.push(shield),
        }
    }
    once_shields
}

fn apply_attribute_update(
    entity: &mut Entity,
    attribute_id: &str,
    value: i64,
    once_shields: &mut Vec<Effect>,
) {
    let adjusted = absorb_with_shields(once_shields, attribute_id, value);
    if let Some(attr) = entity.attributes.get_mut(attribute_id) {
        attr.current_value = (attr.current_value + adjusted)
            .max(attr.min_value)
            .min(attr.max_value);
    }
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

    fn damage_effect(value: i64) -> Effect {
        Effect {
            name: "damage".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
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

    fn attack_ability(damage: i64) -> Ability {
        Ability {
            id: "attack".to_string(),
            name: "Attack".to_string(),
            description: None,
            effects: vec![damage_effect(damage)],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![],
            modifiers: vec![],
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
}
