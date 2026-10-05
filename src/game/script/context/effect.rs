use serde::{Deserialize, Serialize};

use crate::game::component::Effect;

/// A mutable snapshot of an [`Effect`] exposed to scripts (e.g. `effect.effect_type.value =
/// ...`). Unlike [`super::CharacterContext`], this round-trips both ways: a script's mutations
/// are read back via [`super::from_js_value`] into the [`Effect`] that battle resolution
/// consumes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffectContext(Effect);

impl EffectContext {
    pub fn into_effect(self) -> Effect {
        self.0
    }
}

impl From<&Effect> for EffectContext {
    fn from(effect: &Effect) -> Self {
        Self(effect.clone())
    }
}

#[cfg(test)]
mod tests {
    use boa_engine::{Context, Source, js_string};

    use super::*;
    use crate::game::component::{EffectDescription, EffectType, TriggerInfo};

    fn test_effect() -> Effect {
        Effect {
            name: "physical_damage".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -8,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
            scope: Default::default(),
        }
    }

    #[test]
    fn into_effect_returns_the_wrapped_effect() {
        let effect = test_effect();
        let context = EffectContext::from(&effect);
        assert_eq!(context.into_effect(), effect);
    }

    #[test]
    fn script_mutation_round_trips_into_a_modified_effect() {
        let mut js_context = Context::default();
        let context = EffectContext::from(&test_effect());
        let js_value = super::super::to_js_value(&context, &mut js_context).unwrap();
        js_context
            .global_object()
            .set(js_string!("effect"), js_value, true, &mut js_context)
            .unwrap();

        js_context
            .eval(Source::from_bytes("effect.effect_type.value = -40;"))
            .unwrap();

        let mutated = js_context
            .global_object()
            .get(js_string!("effect"), &mut js_context)
            .unwrap();
        let updated: EffectContext =
            super::super::from_js_value(&mutated, &mut js_context).unwrap();
        assert_eq!(
            updated.into_effect().effect_type,
            EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -40,
            }
        );
    }

    #[test]
    fn script_mutation_leaves_other_fields_untouched() {
        let mut js_context = Context::default();
        let context = EffectContext::from(&test_effect());
        let js_value = super::super::to_js_value(&context, &mut js_context).unwrap();
        js_context
            .global_object()
            .set(js_string!("effect"), js_value, true, &mut js_context)
            .unwrap();

        js_context
            .eval(Source::from_bytes("effect.effect_type.value = -40;"))
            .unwrap();

        let mutated = js_context
            .global_object()
            .get(js_string!("effect"), &mut js_context)
            .unwrap();
        let updated: EffectContext =
            super::super::from_js_value(&mutated, &mut js_context).unwrap();
        assert_eq!(updated.into_effect().name, "physical_damage");
    }
}
