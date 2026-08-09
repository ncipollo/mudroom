use crate::game::character::Character;
use crate::game::component::{Ability, AbilityRole};
use crate::game::engagement::EngagementType;

fn battle_abilities(character: &Character) -> Vec<Ability> {
    character
        .innate_abilities
        .iter()
        .filter(|a| a.engagement_types.contains(&EngagementType::Battle))
        .cloned()
        .collect()
}

pub fn entity_innate_battle_abilities(character: &Character) -> Vec<Ability> {
    battle_abilities(character)
}

pub fn battle_attack_abilities(character: &Character) -> Vec<Ability> {
    character
        .innate_abilities
        .iter()
        .filter(|a| {
            a.engagement_types.contains(&EngagementType::Battle) && a.role == AbilityRole::Attack
        })
        .cloned()
        .collect()
}

pub fn battle_defend_abilities(character: &Character) -> Vec<Ability> {
    character
        .innate_abilities
        .iter()
        .filter(|a| {
            a.engagement_types.contains(&EngagementType::Battle) && a.role == AbilityRole::Defend
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::game::character::CharacterType;
    use crate::game::component::Description;
    use crate::game::component::Location;

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
            description: Description::default(),
            effects: vec![],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![],
            modifiers: vec![],
            role,
            targets: vec![],
            action_text: None,
        }
    }

    #[test]
    fn battle_attack_abilities_returns_only_attack_role() {
        let mut character = Character::new(1, CharacterType::Enemy, test_location());
        character.innate_abilities = vec![
            make_ability("slash", AbilityRole::Attack),
            make_ability("shield", AbilityRole::Defend),
        ];
        let attacks = battle_attack_abilities(&character);
        assert_eq!(attacks.len(), 1);
        assert_eq!(attacks[0].id, "slash");
    }

    #[test]
    fn battle_defend_abilities_returns_only_defend_role() {
        let mut character = Character::new(1, CharacterType::Enemy, test_location());
        character.innate_abilities = vec![
            make_ability("slash", AbilityRole::Attack),
            make_ability("shield", AbilityRole::Defend),
        ];
        let defends = battle_defend_abilities(&character);
        assert_eq!(defends.len(), 1);
        assert_eq!(defends[0].id, "shield");
    }

    #[test]
    fn battle_attack_abilities_empty_when_none_match() {
        let mut character = Character::new(1, CharacterType::Enemy, test_location());
        character.innate_abilities = vec![make_ability("shield", AbilityRole::Defend)];
        assert!(battle_attack_abilities(&character).is_empty());
    }

    #[test]
    fn battle_defend_abilities_empty_when_none_match() {
        let mut character = Character::new(1, CharacterType::Enemy, test_location());
        character.innate_abilities = vec![make_ability("slash", AbilityRole::Attack)];
        assert!(battle_defend_abilities(&character).is_empty());
    }
}
