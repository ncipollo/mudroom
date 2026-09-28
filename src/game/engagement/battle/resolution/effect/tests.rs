use super::*;
use crate::game::component::Ability;
use crate::game::component::AbilityRole;
use crate::game::component::Attribute;
use crate::game::component::Description;
use crate::game::component::Location;
use crate::game::component::effect::{EffectDescription, EffectScope};
use crate::game::engagement::EngagementType;
use crate::game::entity::character::CharacterType;

fn config() -> AttributeConfig {
    AttributeConfig::default_config()
}

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

fn buff_effect(attribute_id: &str, value: i64, trigger: TriggerInfo) -> Effect {
    Effect {
        name: "strength_buff".to_string(),
        effect_type: EffectType::AttributeBuff {
            attribute_id: attribute_id.to_string(),
            value,
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
        description: Description::default(),
        effects: vec![damage_effect(damage)],
        engagement_types: vec![EngagementType::Battle],
        costs: vec![],
        modifiers: vec![],
        role: AbilityRole::Attack,
        targets: vec![],
        action_text: None,
    }
}

fn single_entity(hp: i64) -> HashMap<i64, Character> {
    let mut entities = HashMap::new();
    let mut character = Character::new(1, CharacterType::Player, test_location());
    character
        .attributes
        .insert("hp".to_string(), hp_attribute(hp));
    entities.insert(1, character);
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
        &config(),
    );

    let character = entities.get(&1).unwrap();
    assert_eq!(character.attributes["hp"].current_value, 95);
    assert!(character.active_effects.is_empty());
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

    resolve_effects(
        1,
        vec![over_time_shield, damage_effect(-10)],
        &mut entities,
        &config(),
    );

    let character = entities.get(&1).unwrap();
    // OverTime shield pushed to active_effects but not applied — full damage lands
    assert_eq!(character.attributes["hp"].current_value, 90);
    assert_eq!(character.active_effects.len(), 1);
}

#[test]
fn shield_does_not_affect_positive_attribute_updates() {
    let mut entities = single_entity(50);
    let effects = vec![shield_effect(5, TriggerInfo::Once), damage_effect(10)];

    resolve_effects(1, effects, &mut entities, &config());

    let character = entities.get(&1).unwrap();
    assert_eq!(character.attributes["hp"].current_value, 60);
    // Shield is still present since heals don't trigger it
    // (shield was in same-pass effects, not active_effects — it's simply not consumed)
    assert!(character.active_effects.is_empty());
}

#[test]
fn defend_and_attack_in_same_pass_shield_intercepts() {
    let mut entities = single_entity(100);

    // Simulate: defend (shield 5) + attack (-10) queued for same target in same resolution pass
    let effects = vec![shield_effect(5, TriggerInfo::Once), damage_effect(-10)];

    resolve_effects(1, effects, &mut entities, &config());

    let character = entities.get(&1).unwrap();
    // Shield absorbed 5, 5 damage gets through
    assert_eq!(character.attributes["hp"].current_value, 95);
    assert!(character.active_effects.is_empty());
}

#[test]
fn shield_fully_absorbs_attack() {
    let mut entities = single_entity(100);
    let effects = vec![shield_effect(20, TriggerInfo::Once), damage_effect(-10)];

    resolve_effects(1, effects, &mut entities, &config());

    let character = entities.get(&1).unwrap();
    assert_eq!(character.attributes["hp"].current_value, 100);
    assert!(character.active_effects.is_empty());
}

#[test]
fn shield_depletes_across_multiple_hits_until_exhausted() {
    let mut entities = single_entity(100);
    let effects = vec![
        shield_effect(8, TriggerInfo::Once),
        damage_effect(-5),
        damage_effect(-5),
    ];

    resolve_effects(1, effects, &mut entities, &config());

    let character = entities.get(&1).unwrap();
    // Shield absorbs 5 from first hit (3 remaining), then 3 from second (exhausted),
    // leaving 2 damage through
    assert_eq!(character.attributes["hp"].current_value, 98);
    assert!(character.active_effects.is_empty());
}

#[test]
fn absorb_clamps_to_zero_not_positive() {
    let mut entities = single_entity(100);
    let effects = vec![shield_effect(20, TriggerInfo::Once), damage_effect(-10)];

    resolve_effects(1, effects, &mut entities, &config());

    let character = entities.get(&1).unwrap();
    assert_eq!(character.attributes["hp"].current_value, 100);
}

#[test]
fn unknown_target_is_noop() {
    let mut entities: HashMap<i64, Character> = HashMap::new();
    let messages = resolve_effects(99, vec![damage_effect(-10)], &mut entities, &config());
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

    resolve_effects(1, vec![over_time_shield], &mut entities, &config());

    let character = entities.get(&1).unwrap();
    assert_eq!(character.active_effects.len(), 1);
    assert_eq!(character.attributes["hp"].current_value, 100);
}

#[test]
fn attack_ability_effects_resolve_correctly() {
    let mut entities = single_entity(100);
    resolve_effects(1, attack_ability(-10).effects, &mut entities, &config());
    assert_eq!(entities[&1].attributes["hp"].current_value, 90);
}

fn non_updatable_hp_config() -> AttributeConfig {
    let mut config = config();
    config
        .attributes
        .iter_mut()
        .find(|a| a.id == "hp")
        .unwrap()
        .updatable = false;
    config
}

#[test]
fn non_updatable_attribute_is_left_unchanged() {
    let mut entities = single_entity(100);
    resolve_effects(
        1,
        vec![damage_effect(-10)],
        &mut entities,
        &non_updatable_hp_config(),
    );

    let character = entities.get(&1).unwrap();
    assert_eq!(character.attributes["hp"].current_value, 100);
}

#[test]
fn once_triggered_buff_is_pushed_onto_active_effects() {
    let mut entities = single_entity(100);
    resolve_effects(
        1,
        vec![buff_effect("hp", 20, TriggerInfo::Once)],
        &mut entities,
        &config(),
    );

    let character = entities.get(&1).unwrap();
    // Buffs never mutate current_value directly — only the effective read path does.
    assert_eq!(character.attributes["hp"].current_value, 100);
    assert_eq!(character.active_effects.len(), 1);
}

#[test]
fn over_time_buff_is_also_pushed_onto_active_effects() {
    let mut entities = single_entity(100);
    let over_time_buff = buff_effect(
        "hp",
        20,
        TriggerInfo::OverTime {
            start: 0,
            end: Some(3),
            rate: 3,
        },
    );

    resolve_effects(1, vec![over_time_buff], &mut entities, &config());

    let character = entities.get(&1).unwrap();
    assert_eq!(character.attributes["hp"].current_value, 100);
    assert_eq!(character.active_effects.len(), 1);
}

#[test]
fn shield_still_absorbs_before_non_updatable_check_short_circuits() {
    let mut entities = single_entity(100);
    // Shield absorbs 5 of the 10 damage even though hp isn't updatable — absorption runs
    // ahead of the updatable check, it just never gets written to current_value.
    resolve_effects(
        1,
        vec![shield_effect(5, TriggerInfo::Once), damage_effect(-10)],
        &mut entities,
        &non_updatable_hp_config(),
    );

    let character = entities.get(&1).unwrap();
    assert_eq!(character.attributes["hp"].current_value, 100);
    assert!(character.active_effects.is_empty());
}
