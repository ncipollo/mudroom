use crate::game::component::Ability;

#[derive(Debug, Clone, PartialEq)]
pub struct QueuedAbility {
    pub caster_id: i64,
    pub ability: Ability,
    pub target_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::AbilityRole;
    use crate::game::component::effect::{
        Effect, EffectDescription, EffectScope, EffectType, TriggerInfo,
    };
    use crate::game::engagement::EngagementType;

    fn test_ability() -> Ability {
        Ability {
            id: "attack".to_string(),
            name: "Attack".to_string(),
            description: None,
            effects: vec![Effect {
                name: "damage".to_string(),
                effect_type: EffectType::AttributeUpdate {
                    attribute_id: "hp".to_string(),
                    value: -5,
                },
                trigger_info: TriggerInfo::Once,
                description: EffectDescription::default(),
                scope: EffectScope::default(),
            }],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![],
            modifiers: vec![],
            role: AbilityRole::Attack,
            targets: vec![],
            action_text: None,
        }
    }

    #[test]
    fn queued_ability_stores_fields() {
        let ability = test_ability();
        let qa = QueuedAbility {
            caster_id: 1,
            ability: ability.clone(),
            target_id: 2,
        };
        assert_eq!(qa.caster_id, 1);
        assert_eq!(qa.target_id, 2);
        assert_eq!(qa.ability, ability);
    }
}
