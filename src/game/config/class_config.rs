use crate::game::config::ability_config::AbilityReference;
use crate::game::config::entity_config::StartingAttribute;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassConfig {
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub attributes: Vec<StartingAttribute>,
    #[serde(default)]
    pub innate_abilities: Vec<AbilityReference>,
}

pub fn load_classes(config_dir: &Path) -> Result<HashMap<String, ClassConfig>, Box<dyn Error>> {
    let mut configs = HashMap::new();
    let classes_dir = config_dir.join("classes");
    if !classes_dir.exists() {
        return Ok(configs);
    }
    for entry in walkdir::WalkDir::new(&classes_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"))
    {
        let path = entry.path();
        let content = std::fs::read_to_string(path)?;
        let mut config: ClassConfig = toml::from_str(&content)?;
        let id = if let Some(id) = config.id.clone() {
            id
        } else {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string()
        };
        config.id = Some(id.clone());
        configs.insert(id, config);
    }
    Ok(configs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_file(base: &Path, rel: &str, contents: &str) {
        let path = base.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn load_classes_returns_empty_when_no_classes_dir() {
        let tmp = TempDir::new().unwrap();
        let configs = load_classes(tmp.path()).unwrap();
        assert!(configs.is_empty());
    }

    #[test]
    fn load_classes_finds_toml_files() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "classes/warrior.toml", r#"name = "Warrior""#);
        let configs = load_classes(tmp.path()).unwrap();
        assert_eq!(configs.len(), 1);
        assert!(configs.contains_key("warrior"));
    }

    #[test]
    fn load_classes_uses_id_field_when_present() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "classes/myfile.toml",
            r#"
id = "custom_id"
name = "Custom"
"#,
        );
        let configs = load_classes(tmp.path()).unwrap();
        assert!(configs.contains_key("custom_id"));
        assert!(!configs.contains_key("myfile"));
    }

    #[test]
    fn class_config_round_trip() {
        let toml = r#"
name = "Survivor"
description = "A scrappy melee brawler."
innate_abilities = ["basic_attack"]

[[attributes]]
definition_id = "hp"
min_value = 0
max_value = 120
current_value = 120
"#;
        let config: ClassConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.name, "Survivor");
        assert_eq!(config.attributes.len(), 1);
        assert_eq!(config.attributes[0].definition_id, "hp");
        assert_eq!(config.attributes[0].max_value, 120);
        assert_eq!(config.innate_abilities.len(), 1);
        assert_eq!(config.innate_abilities[0].0, "basic_attack");
    }
}
