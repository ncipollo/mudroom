use std::collections::HashMap;

use crate::game::component::{Ability, AbilityRole, ItemDefinition};
use crate::game::engagement::EngagementType;
use crate::game::entity::character::Character;

fn battle_abilities(
    character: &Character,
    item_definitions: &HashMap<String, ItemDefinition>,
    abilities: &HashMap<String, Ability>,
) -> Vec<Ability> {
    character
        .combined_abilities(item_definitions, abilities)
        .into_iter()
        .filter(|a| a.engagement_types.contains(&EngagementType::Battle))
        .collect()
}

pub fn entity_battle_abilities(
    character: &Character,
    item_definitions: &HashMap<String, ItemDefinition>,
    abilities: &HashMap<String, Ability>,
) -> Vec<Ability> {
    battle_abilities(character, item_definitions, abilities)
}

pub fn battle_attack_abilities(
    character: &Character,
    item_definitions: &HashMap<String, ItemDefinition>,
    abilities: &HashMap<String, Ability>,
) -> Vec<Ability> {
    battle_abilities(character, item_definitions, abilities)
        .into_iter()
        .filter(|a| a.role == AbilityRole::Attack)
        .collect()
}

pub fn battle_defend_abilities(
    character: &Character,
    item_definitions: &HashMap<String, ItemDefinition>,
    abilities: &HashMap<String, Ability>,
) -> Vec<Ability> {
    battle_abilities(character, item_definitions, abilities)
        .into_iter()
        .filter(|a| a.role == AbilityRole::Defend)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::game::component::Description;
    use crate::game::component::Location;
    use crate::game::entity::character::CharacterType;

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

    fn no_definitions() -> HashMap<String, ItemDefinition> {
        HashMap::new()
    }

    fn no_abilities() -> HashMap<String, Ability> {
        HashMap::new()
    }

    #[test]
    fn battle_attack_abilities_returns_only_attack_role() {
        let mut character = Character::new(1, CharacterType::Enemy, test_location());
        character.innate_abilities = vec![
            make_ability("slash", AbilityRole::Attack),
            make_ability("shield", AbilityRole::Defend),
        ];
        let attacks = battle_attack_abilities(&character, &no_definitions(), &no_abilities());
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
        let defends = battle_defend_abilities(&character, &no_definitions(), &no_abilities());
        assert_eq!(defends.len(), 1);
        assert_eq!(defends[0].id, "shield");
    }

    #[test]
    fn battle_attack_abilities_empty_when_none_match() {
        let mut character = Character::new(1, CharacterType::Enemy, test_location());
        character.innate_abilities = vec![make_ability("shield", AbilityRole::Defend)];
        assert!(battle_attack_abilities(&character, &no_definitions(), &no_abilities()).is_empty());
    }

    #[test]
    fn battle_defend_abilities_empty_when_none_match() {
        let mut character = Character::new(1, CharacterType::Enemy, test_location());
        character.innate_abilities = vec![make_ability("slash", AbilityRole::Attack)];
        assert!(battle_defend_abilities(&character, &no_definitions(), &no_abilities()).is_empty());
    }

    #[test]
    fn battle_attack_abilities_includes_equipment_granted_ability() {
        use crate::game::component::{EquippedBonuses, Item, ItemUseType};

        let mut character = Character::new(1, CharacterType::Enemy, test_location());
        character.inventory.equipment.insert(
            "weapon".to_string(),
            Item {
                id: 1,
                item_definition_id: "spiked_bat".to_string(),
            },
        );
        let mut definitions = no_definitions();
        definitions.insert(
            "spiked_bat".to_string(),
            ItemDefinition {
                id: "spiked_bat".to_string(),
                name: "Spiked Bat".to_string(),
                description: Description::default(),
                use_type: ItemUseType::Passive,
                item_type: "weapon".to_string(),
                equipped_bonuses: EquippedBonuses {
                    attributes: vec![],
                    equipped: vec!["painful_smash".to_string()],
                },
                use_effects: vec![],
                alternate_names: vec![],
            },
        );
        let mut abilities = no_abilities();
        abilities.insert(
            "painful_smash".to_string(),
            make_ability("painful_smash", AbilityRole::Attack),
        );

        let attacks = battle_attack_abilities(&character, &definitions, &abilities);
        assert_eq!(attacks.len(), 1);
        assert_eq!(attacks[0].id, "painful_smash");
    }
}
