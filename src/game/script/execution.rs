use boa_engine::{Context, Source, js_string};

use crate::game::component::Effect;
use crate::game::entity::character::Character;

use super::context::{CharacterContext, ContextError, EffectContext, from_js_value, to_js_value};

/// Runs `source` with `character` and `effect` injected as the `character`/`effect` JS
/// globals (the convention established by [`CharacterContext`] and [`EffectContext`]'s own
/// tests), returning `effect` as mutated by the script.
pub fn run_on_effect(
    source: &str,
    character: &Character,
    effect: &Effect,
) -> Result<Effect, ContextError> {
    let mut js_context = Context::default();

    let character_js = to_js_value(&CharacterContext::from(character), &mut js_context)?;
    js_context
        .global_object()
        .set(js_string!("character"), character_js, true, &mut js_context)?;

    let effect_js = to_js_value(&EffectContext::from(effect), &mut js_context)?;
    js_context
        .global_object()
        .set(js_string!("effect"), effect_js, true, &mut js_context)?;

    js_context.eval(Source::from_bytes(source))?;

    let mutated = js_context
        .global_object()
        .get(js_string!("effect"), &mut js_context)?;
    let updated: EffectContext = from_js_value(&mutated, &mut js_context)?;
    Ok(updated.into_effect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::effect::EffectScope;
    use crate::game::component::{EffectDescription, EffectType, Location, TriggerInfo};
    use crate::game::entity::character::CharacterType;

    fn test_character(strength: i64) -> Character {
        let location = Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        };
        let mut character = Character::new(1, CharacterType::Player, location);
        character.attributes.insert(
            "strength".to_string(),
            crate::game::component::Attribute::new("strength".to_string(), 1, 20, strength),
        );
        character
    }

    fn test_effect() -> Effect {
        Effect {
            name: "smash_damage".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -18,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
            scope: EffectScope::default(),
        }
    }

    #[test]
    fn run_on_effect_scales_value_from_character_attribute() {
        let character = test_character(14);
        let effect = test_effect();
        let updated = run_on_effect(
            "effect.effect_type.value = -(character.attributes.strength.current_value * 2) | 0;",
            &character,
            &effect,
        )
        .unwrap();
        assert_eq!(
            updated.effect_type,
            EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -28,
            }
        );
    }

    #[test]
    fn run_on_effect_leaves_other_fields_untouched() {
        let character = test_character(14);
        let effect = test_effect();
        let updated = run_on_effect(
            "effect.effect_type.value = -(character.attributes.strength.current_value * 2) | 0;",
            &character,
            &effect,
        )
        .unwrap();
        assert_eq!(updated.name, "smash_damage");
    }

    #[test]
    fn run_on_effect_errors_on_invalid_script() {
        let character = test_character(14);
        let effect = test_effect();
        let result = run_on_effect("function (", &character, &effect);
        assert!(result.is_err());
    }
}
