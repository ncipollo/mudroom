use crate::game::component::effect::{Effect, EffectType};
use crate::game::narration::TextResolver;

pub fn effect_text(effect: &Effect) -> String {
    let raw = match &effect.description.text {
        Some(text) => text.as_str(),
        None => default_text(&effect.effect_type),
    };
    if raw.is_empty() {
        return String::new();
    }
    TextResolver::resolve(raw, &effect.effect_type.variable_map())
}

fn default_text(effect_type: &EffectType) -> &'static str {
    if let EffectType::AttributeUpdate {
        attribute_id,
        value,
    } = effect_type
        && attribute_id == "hp"
    {
        return if *value > 0 {
            "heals for {{value}}"
        } else if *value < 0 {
            "deals {{abs_value}} damage"
        } else {
            ""
        };
    }
    ""
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::effect::{EffectDescription, EffectScope, TriggerInfo};

    fn make_effect(effect_type: EffectType, text: Option<&str>) -> Effect {
        Effect {
            name: "test".to_string(),
            effect_type,
            trigger_info: TriggerInfo::Once,
            description: EffectDescription {
                text: text.map(|s| s.to_string()),
                ..Default::default()
            },
            scope: EffectScope::default(),
        }
    }

    #[test]
    fn hp_damage_returns_deals_damage() {
        let e = make_effect(
            EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -10,
            },
            None,
        );
        assert_eq!(effect_text(&e), "deals 10 damage");
    }

    #[test]
    fn hp_healing_returns_heals_for() {
        let e = make_effect(
            EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: 5,
            },
            None,
        );
        assert_eq!(effect_text(&e), "heals for 5");
    }

    #[test]
    fn hp_zero_returns_empty() {
        let e = make_effect(
            EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: 0,
            },
            None,
        );
        assert_eq!(effect_text(&e), "");
    }

    #[test]
    fn non_hp_attribute_returns_empty() {
        let e = make_effect(
            EffectType::AttributeUpdate {
                attribute_id: "mp".to_string(),
                value: -5,
            },
            None,
        );
        assert_eq!(effect_text(&e), "");
    }

    #[test]
    fn author_text_literal_returned_unchanged() {
        let e = make_effect(
            EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -10,
            },
            Some("cleaves for 15"),
        );
        assert_eq!(effect_text(&e), "cleaves for 15");
    }

    #[test]
    fn author_text_with_abs_value_variable_resolved() {
        let e = make_effect(
            EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -10,
            },
            Some("Axe chops for {{abs_value}}"),
        );
        assert_eq!(effect_text(&e), "Axe chops for 10");
    }

    #[test]
    fn author_text_with_value_variable_resolved() {
        let e = make_effect(
            EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: 8,
            },
            Some("restores {{value}} hp"),
        );
        assert_eq!(effect_text(&e), "restores 8 hp");
    }
}
