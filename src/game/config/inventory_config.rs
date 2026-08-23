use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// The well-known inventory config id every mud is expected to define. Character-owned
/// inventories default to this type, and `InventoryConfig::resolve` falls back to it when a
/// character's stored type doesn't match any configured definition.
pub const DEFAULT_INVENTORY_TYPE: &str = "inventory";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentSlotConfig {
    pub name: String,
    #[serde(default)]
    pub item_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryDefinition {
    pub id: String,
    pub bag_size: usize,
    #[serde(default)]
    pub equipment_slots: Vec<EquipmentSlotConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryConfig {
    pub inventories: Vec<InventoryDefinition>,
}

impl InventoryDefinition {
    /// Equipment slots this inventory defines that accept `item_type`.
    pub fn eligible_slots(&self, item_type: &str) -> Vec<&EquipmentSlotConfig> {
        self.equipment_slots
            .iter()
            .filter(|slot| slot.item_types.iter().any(|t| t == item_type))
            .collect()
    }
}

impl InventoryConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn default_config() -> Self {
        Self {
            inventories: vec![InventoryDefinition {
                id: DEFAULT_INVENTORY_TYPE.to_string(),
                bag_size: 20,
                equipment_slots: vec![
                    EquipmentSlotConfig {
                        name: "weapon".to_string(),
                        item_types: vec!["weapon".to_string()],
                    },
                    EquipmentSlotConfig {
                        name: "armor".to_string(),
                        item_types: vec!["armor".to_string()],
                    },
                ],
            }],
        }
    }

    pub fn get(&self, id: &str) -> Option<&InventoryDefinition> {
        self.inventories.iter().find(|def| def.id == id)
    }

    /// Resolves `inventory_type` to its definition, falling back to the well-known
    /// [`DEFAULT_INVENTORY_TYPE`] config (with a warning) when the type is unrecognized.
    pub fn resolve(&self, inventory_type: &str) -> Option<&InventoryDefinition> {
        if let Some(def) = self.get(inventory_type) {
            return Some(def);
        }
        tracing::warn!(
            "Unknown inventory type '{inventory_type}', falling back to '{DEFAULT_INVENTORY_TYPE}'"
        );
        self.get(DEFAULT_INVENTORY_TYPE)
    }

    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        for def in &self.inventories {
            let mut seen = HashSet::new();
            for slot in &def.equipment_slots {
                if !seen.insert(slot.name.as_str()) {
                    return Err(format!(
                        "inventory config '{}' has duplicate equipment slot '{}'",
                        def.id, slot.name
                    )
                    .into());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn default_config_has_expected_shape() {
        let config = InventoryConfig::default_config();
        assert_eq!(config.inventories.len(), 1);
        let def = config.get(DEFAULT_INVENTORY_TYPE).unwrap();
        assert_eq!(def.bag_size, 20);
        assert_eq!(def.equipment_slots.len(), 2);
        assert!(def.equipment_slots.iter().any(|s| s.name == "weapon"));
        assert!(def.equipment_slots.iter().any(|s| s.name == "armor"));
    }

    #[test]
    fn load_parses_toml() {
        let toml = r#"
[[inventories]]
id = "inventory"
bag_size = 10

[[inventories.equipment_slots]]
name = "weapon"
item_types = ["weapon"]

[[inventories.equipment_slots]]
name = "armor"
item_types = ["armor"]
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(toml.as_bytes()).unwrap();
        let config = InventoryConfig::load(file.path()).unwrap();
        assert_eq!(config.inventories.len(), 1);
        let def = &config.inventories[0];
        assert_eq!(def.id, "inventory");
        assert_eq!(def.bag_size, 10);
        assert_eq!(def.equipment_slots.len(), 2);
    }

    #[test]
    fn load_parses_multiple_named_configs() {
        let toml = r#"
[[inventories]]
id = "inventory"
bag_size = 10

[[inventories]]
id = "merchant_stall"
bag_size = 100
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(toml.as_bytes()).unwrap();
        let config = InventoryConfig::load(file.path()).unwrap();
        assert_eq!(config.inventories.len(), 2);
        assert!(config.get("merchant_stall").is_some());
    }

    #[test]
    fn load_fails_on_duplicate_slot_names() {
        let toml = r#"
[[inventories]]
id = "inventory"
bag_size = 10

[[inventories.equipment_slots]]
name = "weapon"
item_types = ["weapon"]

[[inventories.equipment_slots]]
name = "weapon"
item_types = ["ranged"]
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(toml.as_bytes()).unwrap();
        let result = InventoryConfig::load(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn resolve_returns_matching_definition() {
        let config = InventoryConfig::default_config();
        let def = config.resolve(DEFAULT_INVENTORY_TYPE).unwrap();
        assert_eq!(def.id, DEFAULT_INVENTORY_TYPE);
    }

    #[test]
    fn resolve_falls_back_to_default_for_unknown_type() {
        let config = InventoryConfig::default_config();
        let def = config.resolve("nonexistent").unwrap();
        assert_eq!(def.id, DEFAULT_INVENTORY_TYPE);
    }

    #[test]
    fn eligible_slots_returns_matching_slots() {
        let def = InventoryConfig::default_config()
            .get(DEFAULT_INVENTORY_TYPE)
            .unwrap()
            .clone();
        let slots = def.eligible_slots("weapon");
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].name, "weapon");
    }

    #[test]
    fn eligible_slots_is_empty_for_unknown_item_type() {
        let def = InventoryConfig::default_config()
            .get(DEFAULT_INVENTORY_TYPE)
            .unwrap()
            .clone();
        assert!(def.eligible_slots("amulet").is_empty());
    }
}
