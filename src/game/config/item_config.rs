pub mod config;
pub mod loader;

pub use config::{AttributeBonus, EquippedBonuses, ItemDefinition, ItemUseType, UseEffect};
pub use loader::load_item;
