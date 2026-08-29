use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::game::component::{Ability, AttributeBonus, Item, ItemDefinition};
use crate::game::config::inventory_config::DEFAULT_INVENTORY_TYPE;

fn default_inventory_type() -> String {
    DEFAULT_INVENTORY_TYPE.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    #[serde(default = "default_inventory_type")]
    pub inventory_type: String,
    #[serde(default)]
    pub bag: Vec<Item>,
    #[serde(default)]
    pub equipment: HashMap<String, Item>,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            inventory_type: default_inventory_type(),
            bag: Vec::new(),
            equipment: HashMap::new(),
        }
    }
}

impl Inventory {
    pub fn stat_bonuses(
        &self,
        item_definitions: &HashMap<String, ItemDefinition>,
    ) -> Vec<AttributeBonus> {
        self.equipped_definitions(item_definitions)
            .flat_map(|def| def.equipped_bonuses.attributes.clone())
            .collect()
    }

    pub fn granted_abilities(
        &self,
        item_definitions: &HashMap<String, ItemDefinition>,
        abilities: &HashMap<String, Ability>,
    ) -> Vec<Ability> {
        self.equipped_definitions(item_definitions)
            .flat_map(|def| def.equipped_bonuses.equipped.iter())
            .filter_map(|ability_id| abilities.get(ability_id).cloned())
            .collect()
    }

    fn equipped_definitions<'a>(
        &'a self,
        item_definitions: &'a HashMap<String, ItemDefinition>,
    ) -> impl Iterator<Item = &'a ItemDefinition> {
        self.equipment
            .values()
            .filter_map(|item| item_definitions.get(&item.item_definition_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::description::Description;
    use crate::game::component::{AbilityRole, EquippedBonuses, ItemUseType};
    use crate::game::engagement::EngagementType;

    fn vest_definition() -> ItemDefinition {
        ItemDefinition {
            id: "leather_vest".to_string(),
            name: "Leather Vest".to_string(),
            description: Description::default(),
            use_type: ItemUseType::Passive,
            item_type: "armor".to_string(),
            equipped_bonuses: EquippedBonuses {
                attributes: vec![AttributeBonus {
                    attribute_id: "constitution".to_string(),
                    amount: 2,
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

    fn painful_smash_ability() -> Ability {
        Ability {
            id: "painful_smash".to_string(),
            name: "Painful Smash".to_string(),
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
    fn default_inventory_type_is_well_known() {
        let inventory = Inventory::default();
        assert_eq!(inventory.inventory_type, DEFAULT_INVENTORY_TYPE);
        assert!(inventory.bag.is_empty());
        assert!(inventory.equipment.is_empty());
    }

    #[test]
    fn stat_bonuses_resolves_equipped_definitions() {
        let mut definitions = HashMap::new();
        definitions.insert("leather_vest".to_string(), vest_definition());

        let inventory = Inventory {
            equipment: HashMap::from([(
                "armor".to_string(),
                Item {
                    id: 1,
                    item_definition_id: "leather_vest".to_string(),
                },
            )]),
            ..Inventory::default()
        };

        let bonuses = inventory.stat_bonuses(&definitions);
        assert_eq!(bonuses.len(), 1);
        assert_eq!(bonuses[0].attribute_id, "constitution");
        assert_eq!(bonuses[0].amount, 2);
    }

    #[test]
    fn stat_bonuses_ignores_items_missing_a_definition() {
        let definitions = HashMap::new();
        let inventory = Inventory {
            equipment: HashMap::from([(
                "armor".to_string(),
                Item {
                    id: 1,
                    item_definition_id: "unknown".to_string(),
                },
            )]),
            ..Inventory::default()
        };

        assert!(inventory.stat_bonuses(&definitions).is_empty());
    }

    #[test]
    fn granted_abilities_resolves_equipped_definitions() {
        let mut definitions = HashMap::new();
        definitions.insert("spiked_bat".to_string(), bat_definition());
        let mut abilities = HashMap::new();
        abilities.insert("painful_smash".to_string(), painful_smash_ability());

        let inventory = Inventory {
            equipment: HashMap::from([(
                "weapon".to_string(),
                Item {
                    id: 1,
                    item_definition_id: "spiked_bat".to_string(),
                },
            )]),
            ..Inventory::default()
        };

        let granted = inventory.granted_abilities(&definitions, &abilities);
        assert_eq!(granted.len(), 1);
        assert_eq!(granted[0].id, "painful_smash");
    }

    #[test]
    fn granted_abilities_ignores_unresolvable_ability_ids() {
        let mut definitions = HashMap::new();
        definitions.insert("spiked_bat".to_string(), bat_definition());
        let abilities = HashMap::new();

        let inventory = Inventory {
            equipment: HashMap::from([(
                "weapon".to_string(),
                Item {
                    id: 1,
                    item_definition_id: "spiked_bat".to_string(),
                },
            )]),
            ..Inventory::default()
        };

        assert!(
            inventory
                .granted_abilities(&definitions, &abilities)
                .is_empty()
        );
    }
}
