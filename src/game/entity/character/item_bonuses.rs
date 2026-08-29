use std::collections::HashMap;

use crate::game::component::{Ability, Attribute, AttributeBonus, ItemDefinition};

use super::Character;

impl Character {
    /// Base attributes plus additive bonuses from every contributing source (currently just
    /// equipped items), folded in a fixed order so a future source (e.g. skills) is a one-place
    /// addition here rather than a call-site change.
    pub fn combined_attributes(
        &self,
        item_definitions: &HashMap<String, ItemDefinition>,
    ) -> HashMap<String, Attribute> {
        let mut combined = self.attributes.clone();
        for bonus in self.inventory.stat_bonuses(item_definitions) {
            apply_bonus(&mut combined, &bonus);
        }
        combined
    }

    /// Innate abilities plus abilities granted by every contributing source (currently just
    /// equipped items), deduplicated by id and folded in the same fixed-order fashion as
    /// `combined_attributes`.
    pub fn combined_abilities(
        &self,
        item_definitions: &HashMap<String, ItemDefinition>,
        abilities: &HashMap<String, Ability>,
    ) -> Vec<Ability> {
        let mut combined = self.innate_abilities.clone();
        for ability in self
            .inventory
            .granted_abilities(item_definitions, abilities)
        {
            push_unique(&mut combined, ability);
        }
        combined
    }
}

fn apply_bonus(attributes: &mut HashMap<String, Attribute>, bonus: &AttributeBonus) {
    if let Some(attribute) = attributes.get_mut(&bonus.attribute_id) {
        attribute.current_value = (attribute.current_value + bonus.amount)
            .clamp(attribute.min_value, attribute.max_value);
    }
}

fn push_unique(abilities: &mut Vec<Ability>, ability: Ability) {
    if !abilities.iter().any(|a| a.id == ability.id) {
        abilities.push(ability);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::Location;
    use crate::game::component::description::Description;
    use crate::game::component::{AbilityRole, EquippedBonuses, Item, ItemUseType};
    use crate::game::engagement::EngagementType;
    use crate::game::entity::character::CharacterType;

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    fn make_character() -> Character {
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character.attributes.insert(
            "constitution".to_string(),
            Attribute::new("constitution".to_string(), 0, 20, 10),
        );
        character
    }

    fn vest_definition(bonus_amount: i64) -> ItemDefinition {
        ItemDefinition {
            id: "leather_vest".to_string(),
            name: "Leather Vest".to_string(),
            description: Description::default(),
            use_type: ItemUseType::Passive,
            item_type: "armor".to_string(),
            equipped_bonuses: EquippedBonuses {
                attributes: vec![AttributeBonus {
                    attribute_id: "constitution".to_string(),
                    amount: bonus_amount,
                }],
                equipped: vec![],
            },
            use_effects: vec![],
            alternate_names: vec![],
        }
    }

    fn bat_definition() -> ItemDefinition {
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
        }
    }

    fn make_ability(id: &str) -> Ability {
        Ability {
            id: id.to_string(),
            name: id.to_string(),
            description: Description::default(),
            effects: vec![],
            costs: vec![],
            modifiers: vec![],
            engagement_types: vec![EngagementType::Battle],
            role: AbilityRole::Attack,
            targets: vec![],
            action_text: None,
        }
    }

    #[test]
    fn combined_attributes_passes_through_when_no_equipment() {
        let character = make_character();
        let combined = character.combined_attributes(&HashMap::new());
        assert_eq!(combined["constitution"].current_value, 10);
    }

    #[test]
    fn combined_attributes_applies_equipped_bonus() {
        let mut character = make_character();
        character.inventory.equipment.insert(
            "armor".to_string(),
            Item {
                id: 1,
                item_definition_id: "leather_vest".to_string(),
            },
        );
        let mut definitions = HashMap::new();
        definitions.insert("leather_vest".to_string(), vest_definition(2));

        let combined = character.combined_attributes(&definitions);
        assert_eq!(combined["constitution"].current_value, 12);
    }

    #[test]
    fn combined_attributes_clamps_bonus_to_max_value() {
        let mut character = make_character();
        character.inventory.equipment.insert(
            "armor".to_string(),
            Item {
                id: 1,
                item_definition_id: "leather_vest".to_string(),
            },
        );
        let mut definitions = HashMap::new();
        definitions.insert("leather_vest".to_string(), vest_definition(50));

        let combined = character.combined_attributes(&definitions);
        assert_eq!(combined["constitution"].current_value, 20);
    }

    #[test]
    fn combined_attributes_ignores_bonus_for_unknown_attribute() {
        let mut character = make_character();
        character.inventory.equipment.insert(
            "accessory".to_string(),
            Item {
                id: 1,
                item_definition_id: "ring".to_string(),
            },
        );
        let mut definitions = HashMap::new();
        definitions.insert(
            "ring".to_string(),
            ItemDefinition {
                id: "ring".to_string(),
                name: "Ring".to_string(),
                description: Description::default(),
                use_type: ItemUseType::Passive,
                item_type: "ring".to_string(),
                equipped_bonuses: EquippedBonuses {
                    attributes: vec![AttributeBonus {
                        attribute_id: "luck".to_string(),
                        amount: 5,
                    }],
                    equipped: vec![],
                },
                use_effects: vec![],
                alternate_names: vec![],
            },
        );

        let combined = character.combined_attributes(&definitions);
        assert_eq!(combined["constitution"].current_value, 10);
        assert!(!combined.contains_key("luck"));
    }

    #[test]
    fn combined_abilities_concatenates_innate_and_granted() {
        let mut character = make_character();
        character.innate_abilities = vec![make_ability("slash")];
        character.inventory.equipment.insert(
            "weapon".to_string(),
            Item {
                id: 1,
                item_definition_id: "spiked_bat".to_string(),
            },
        );
        let mut definitions = HashMap::new();
        definitions.insert("spiked_bat".to_string(), bat_definition());
        let mut abilities = HashMap::new();
        abilities.insert("painful_smash".to_string(), make_ability("painful_smash"));

        let combined = character.combined_abilities(&definitions, &abilities);
        let ids: Vec<&str> = combined.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, vec!["slash", "painful_smash"]);
    }

    #[test]
    fn combined_abilities_dedupes_when_granted_ability_is_also_innate() {
        let mut character = make_character();
        character.innate_abilities = vec![make_ability("painful_smash")];
        character.inventory.equipment.insert(
            "weapon".to_string(),
            Item {
                id: 1,
                item_definition_id: "spiked_bat".to_string(),
            },
        );
        let mut definitions = HashMap::new();
        definitions.insert("spiked_bat".to_string(), bat_definition());
        let mut abilities = HashMap::new();
        abilities.insert("painful_smash".to_string(), make_ability("painful_smash"));

        let combined = character.combined_abilities(&definitions, &abilities);
        assert_eq!(combined.len(), 1);
    }
}
