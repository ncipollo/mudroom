use crate::game::engagement::battle::abilities::{
    battle_attack_abilities, battle_defend_abilities,
};
use crate::game::entity::character::Character;

use super::decision::AiDecision;
use super::pick_random;

pub fn plan_attack(character: &Character, targets: &[i64]) -> AiDecision {
    let Some(&target_id) = pick_random(targets) else {
        return AiDecision::Skip(character.id);
    };
    let attacks = battle_attack_abilities(character);
    match pick_random(&attacks).cloned() {
        Some(ability) => AiDecision::Action(Box::new((
            character.id,
            ability,
            target_id,
            character.attributes.clone(),
        ))),
        None => AiDecision::Skip(character.id),
    }
}

pub fn plan_defend(character: &Character) -> AiDecision {
    let defends = battle_defend_abilities(character);
    match pick_random(&defends).cloned() {
        Some(ability) => AiDecision::Action(Box::new((
            character.id,
            ability,
            character.id,
            character.attributes.clone(),
        ))),
        None => AiDecision::Skip(character.id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::Ability;
    use crate::game::component::AbilityRole;
    use crate::game::component::Description;
    use crate::game::component::Location;
    use crate::game::component::effect::{
        Effect, EffectDescription, EffectScope, EffectType, TriggerInfo,
    };
    use crate::game::engagement::EngagementType;
    use crate::game::entity::character::CharacterType;

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
            description: Description::default(),
            effects: vec![Effect {
                name: "dmg".to_string(),
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
            role,
            targets: vec![],
            action_text: None,
        }
    }

    fn make_enemy(id: i64) -> Character {
        Character::new(id, CharacterType::Enemy, test_location())
    }

    #[test]
    fn plan_attack_returns_action_with_target() {
        let mut character = make_enemy(1);
        character.innate_abilities = vec![make_ability("slash", AbilityRole::Attack)];
        let targets = vec![2_i64];
        let decision = plan_attack(&character, &targets);
        assert!(matches!(decision, AiDecision::Action(b) if b.0 == 1 && b.2 == 2));
    }

    #[test]
    fn plan_attack_skips_when_no_attack_abilities() {
        let character = make_enemy(1);
        let targets = vec![2_i64];
        assert!(matches!(
            plan_attack(&character, &targets),
            AiDecision::Skip(1)
        ));
    }

    #[test]
    fn plan_attack_skips_when_no_targets() {
        let character = make_enemy(1);
        assert!(matches!(plan_attack(&character, &[]), AiDecision::Skip(1)));
    }

    #[test]
    fn plan_defend_returns_action_targeting_self() {
        let mut character = make_enemy(1);
        character.innate_abilities = vec![make_ability("shield", AbilityRole::Defend)];
        let decision = plan_defend(&character);
        assert!(matches!(decision, AiDecision::Action(b) if b.0 == 1 && b.2 == 1));
    }

    #[test]
    fn plan_defend_skips_when_no_defend_abilities() {
        let character = make_enemy(1);
        assert!(matches!(plan_defend(&character), AiDecision::Skip(1)));
    }
}
