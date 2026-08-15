pub mod attribute;
pub mod attribute_definition;
pub mod description;
pub mod effect;
pub mod faction;
pub mod faction_relations;
pub mod interaction;
pub mod inventory;
pub mod item;
pub mod location;
pub mod modifier;
pub mod resource_definition;

pub use crate::game::config::ability_config::{Ability, AbilityRole, AbilityTargetType, Cost};
pub use crate::game::config::item_config::{
    AttributeBonus, EquippedBonuses, ItemDefinition, ItemUseType, UseEffect,
};
pub use attribute::Attribute;
pub use attribute_definition::AttributeCategory;
pub use attribute_definition::AttributeDefinition;
pub use attribute_definition::AttributeType;
pub use attribute_definition::ResetCondition;
pub use description::Description;
pub use effect::Effect;
pub use effect::EffectDescription;
pub use effect::EffectType;
pub use effect::TriggerInfo;
pub use faction::Faction;
pub use faction_relations::FactionRelation;
pub use faction_relations::FactionRelations;
pub use interaction::Direction;
pub use interaction::Interaction;
pub use interaction::Movement;
pub use inventory::Inventory;
pub use item::Item;
pub use location::Location;
pub use modifier::{Modifier, Operator};
pub use resource_definition::ResourceDefinition;
pub use resource_definition::ResourceLifespan;
