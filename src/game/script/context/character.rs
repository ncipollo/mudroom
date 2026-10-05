use std::collections::HashMap;

use serde::Serialize;

use crate::game::component::{Attribute, Effect, Location};
use crate::game::entity::character::Character;

/// A read-only snapshot of a [`Character`] exposed to scripts as JS-visible properties
/// (attributes, active effects, name, location) — see [`super::to_js_value`]. Unlike
/// [`super::EffectContext`], scripts never write back to this context, so it has no
/// `Deserialize` impl.
#[derive(Debug, Clone, Serialize)]
pub struct CharacterContext {
    pub name: String,
    pub attributes: HashMap<String, Attribute>,
    pub active_effects: Vec<Effect>,
    pub location: Location,
}

impl From<&Character> for CharacterContext {
    fn from(character: &Character) -> Self {
        Self {
            name: character.name.clone(),
            attributes: character.attributes.clone(),
            active_effects: character.active_effects.clone(),
            location: character.location.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use boa_engine::{Context, Source, js_string};
    use serde_json::json;

    use super::*;
    use crate::game::component::{EffectDescription, EffectType, TriggerInfo};
    use crate::game::entity::character::CharacterType;

    fn test_character() -> Character {
        let location = Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "tavern".to_string(),
        };
        let mut character = Character::new(1, CharacterType::Player, location);
        character.name = "Aragorn".to_string();
        character.attributes.insert(
            "strength".to_string(),
            Attribute::new("strength".to_string(), 1, 20, 15),
        );
        character.active_effects.push(Effect {
            name: "strength_buff".to_string(),
            effect_type: EffectType::AttributeBuff {
                attribute_id: "strength".to_string(),
                value: 5,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
            scope: Default::default(),
        });
        character
    }

    fn eval_character_property(character: &CharacterContext, property: &str) -> serde_json::Value {
        let mut js_context = Context::default();
        let js_value = super::super::to_js_value(character, &mut js_context).unwrap();
        js_context
            .global_object()
            .set(js_string!("character"), js_value, true, &mut js_context)
            .unwrap();
        let result = js_context.eval(Source::from_bytes(property)).unwrap();
        result.to_json(&mut js_context).unwrap().unwrap()
    }

    #[test]
    fn from_character_copies_name_attributes_effects_and_location() {
        let character = test_character();
        let context = CharacterContext::from(&character);
        assert_eq!(context.name, "Aragorn");
        assert_eq!(context.attributes["strength"].current_value, 15);
        assert_eq!(context.active_effects.len(), 1);
        assert_eq!(context.location.room_id, "tavern");
    }

    #[test]
    fn js_can_read_character_name() {
        let context = CharacterContext::from(&test_character());
        assert_eq!(
            eval_character_property(&context, "character.name"),
            json!("Aragorn")
        );
    }

    #[test]
    fn js_can_read_character_attribute_current_value() {
        let context = CharacterContext::from(&test_character());
        assert_eq!(
            eval_character_property(&context, "character.attributes.strength.current_value"),
            json!(15)
        );
    }

    #[test]
    fn js_can_read_character_location() {
        let context = CharacterContext::from(&test_character());
        assert_eq!(
            eval_character_property(&context, "character.location.room_id"),
            json!("tavern")
        );
    }

    #[test]
    fn js_can_read_character_active_effect_name() {
        let context = CharacterContext::from(&test_character());
        assert_eq!(
            eval_character_property(&context, "character.active_effects[0].name"),
            json!("strength_buff")
        );
    }
}
