use crate::game::component::Ability;
use crate::game::component::effect::{Effect, EffectDescription, EffectType, TriggerInfo};
use crate::game::engagement::EngagementType;
use crate::game::entity::Entity;

fn battle_abilities(entity: &Entity) -> Vec<Ability> {
    entity
        .innate_abilities
        .iter()
        .filter(|a| a.engagement_types.contains(&EngagementType::Battle))
        .cloned()
        .collect()
}

pub fn entity_innate_battle_abilities(entity: &Entity) -> Vec<Ability> {
    battle_abilities(entity)
}

pub fn default_defend_ability() -> Ability {
    Ability {
        id: "defend".to_string(),
        name: "Defend".to_string(),
        description: Some("Brace for impact, reducing incoming damage by 5.".to_string()),
        effects: vec![Effect {
            name: "damage_reduction".to_string(),
            effect_type: EffectType::AttributeShield {
                attribute_id: "hp".to_string(),
                absorb_amount: 5,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
        }],
        engagement_types: vec![EngagementType::Battle],
        costs: vec![],
        modifiers: vec![],
    }
}

pub fn default_attack_ability() -> Ability {
    Ability {
        id: "attack".to_string(),
        name: "Attack".to_string(),
        description: Some("A basic physical attack.".to_string()),
        effects: vec![Effect {
            name: "physical_damage".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -10,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
        }],
        engagement_types: vec![EngagementType::Battle],
        costs: vec![],
        modifiers: vec![],
    }
}

#[cfg(test)]
mod tests {
    use crate::game::component::effect::{EffectType, TriggerInfo};

    use super::*;

    #[test]
    fn default_defend_ability_has_correct_structure() {
        let ability = default_defend_ability();
        assert_eq!(ability.id, "defend");
        assert_eq!(ability.name, "Defend");
        assert_eq!(ability.effects.len(), 1);
        assert!(matches!(
            ability.effects[0].effect_type,
            EffectType::AttributeShield {
                ref attribute_id,
                absorb_amount: 5,
            } if attribute_id == "hp"
        ));
        assert!(matches!(ability.effects[0].trigger_info, TriggerInfo::Once));
    }
}
