use crate::game::component::effect::{Effect, EffectDescription, EffectType, TriggerInfo};
use crate::game::component::{Ability, AbilityRole};
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

pub fn battle_attack_abilities(entity: &Entity) -> Vec<Ability> {
    entity
        .innate_abilities
        .iter()
        .filter(|a| {
            a.engagement_types.contains(&EngagementType::Battle) && a.role == AbilityRole::Attack
        })
        .cloned()
        .collect()
}

pub fn battle_defend_abilities(entity: &Entity) -> Vec<Ability> {
    entity
        .innate_abilities
        .iter()
        .filter(|a| {
            a.engagement_types.contains(&EngagementType::Battle) && a.role == AbilityRole::Defend
        })
        .cloned()
        .collect()
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
        role: AbilityRole::Defend,
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
        role: AbilityRole::Attack,
    }
}

#[cfg(test)]
mod tests {
    use crate::game::component::Location;
    use crate::game::component::effect::{EffectType, TriggerInfo};
    use crate::game::entity::EntityType;

    use super::*;

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    fn make_ability(id: &str, role: AbilityRole) -> Ability {
        Ability {
            id: id.to_string(),
            name: id.to_string(),
            description: None,
            effects: vec![],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![],
            modifiers: vec![],
            role,
        }
    }

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

    #[test]
    fn battle_attack_abilities_returns_only_attack_role() {
        let mut entity = Entity::new(1, EntityType::Enemy, test_location());
        entity.innate_abilities = vec![
            make_ability("slash", AbilityRole::Attack),
            make_ability("shield", AbilityRole::Defend),
        ];
        let attacks = battle_attack_abilities(&entity);
        assert_eq!(attacks.len(), 1);
        assert_eq!(attacks[0].id, "slash");
    }

    #[test]
    fn battle_defend_abilities_returns_only_defend_role() {
        let mut entity = Entity::new(1, EntityType::Enemy, test_location());
        entity.innate_abilities = vec![
            make_ability("slash", AbilityRole::Attack),
            make_ability("shield", AbilityRole::Defend),
        ];
        let defends = battle_defend_abilities(&entity);
        assert_eq!(defends.len(), 1);
        assert_eq!(defends[0].id, "shield");
    }

    #[test]
    fn battle_attack_abilities_empty_when_none_match() {
        let mut entity = Entity::new(1, EntityType::Enemy, test_location());
        entity.innate_abilities = vec![make_ability("shield", AbilityRole::Defend)];
        assert!(battle_attack_abilities(&entity).is_empty());
    }

    #[test]
    fn battle_defend_abilities_empty_when_none_match() {
        let mut entity = Entity::new(1, EntityType::Enemy, test_location());
        entity.innate_abilities = vec![make_ability("slash", AbilityRole::Attack)];
        assert!(battle_defend_abilities(&entity).is_empty());
    }
}
