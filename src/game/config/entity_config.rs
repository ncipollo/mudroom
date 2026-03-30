use crate::game::component::effect::Effect;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

fn default_agent_type() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityTypeConfig {
    Character,
    Object,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum PersonaConfig {
    Agent {
        #[serde(default = "default_agent_type")]
        agent_type: String,
        persona_file: Option<String>,
    },
    Standard {
        dialog_tree: Option<DialogLine>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogLine {
    pub text: String,
    #[serde(default)]
    pub responses: Vec<PlayerResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerResponse {
    pub text: String,
    pub reply: Option<Box<DialogLine>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartingAttribute {
    pub definition_id: String,
    pub min_value: i64,
    pub max_value: i64,
    pub current_value: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityConfig {
    pub id: Option<String>,
    pub entity_type: EntityTypeConfig,
    #[serde(default)]
    pub description: Option<String>,
    pub persona: Option<PersonaConfig>,
    #[serde(default)]
    pub attributes: Vec<StartingAttribute>,
    #[serde(default)]
    pub entity_effects: Vec<Effect>,
}

pub fn load_entity_config(path: &Path) -> Result<EntityConfig, Box<dyn Error>> {
    let content = std::fs::read_to_string(path)?;
    let config: EntityConfig = toml::from_str(&content)?;
    Ok(config)
}

pub fn load_entity_configs(
    config_dir: &Path,
) -> Result<HashMap<String, EntityConfig>, Box<dyn Error>> {
    let mut configs = HashMap::new();
    let entities_dir = config_dir.join("entities");
    if !entities_dir.exists() {
        return Ok(configs);
    }
    for entry in walkdir::WalkDir::new(&entities_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"))
    {
        let path = entry.path();
        let mut config = load_entity_config(path)?;
        let id = if let Some(id) = config.id.clone() {
            id
        } else {
            let rel = path.strip_prefix(config_dir)?.with_extension("");
            rel.to_string_lossy().to_string()
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
    fn character_config_round_trip() {
        let toml = r#"
entity_type = "character"

[persona]
type = "standard"

[persona.dialog_tree]
text = "Hello!"

[[persona.dialog_tree.responses]]
text = "Just passing through."

[[attributes]]
definition_id = "hp"
min_value = 0
max_value = 100
current_value = 100
"#;
        let config: EntityConfig = toml::from_str(toml).unwrap();
        assert!(matches!(config.entity_type, EntityTypeConfig::Character));
        assert_eq!(config.attributes.len(), 1);
        assert_eq!(config.attributes[0].definition_id, "hp");
        if let Some(PersonaConfig::Standard {
            dialog_tree: Some(tree),
        }) = &config.persona
        {
            assert_eq!(tree.text, "Hello!");
            assert_eq!(tree.responses.len(), 1);
            assert_eq!(tree.responses[0].text, "Just passing through.");
            assert!(tree.responses[0].reply.is_none());
        } else {
            panic!("expected Standard persona with dialog_tree");
        }
    }

    #[test]
    fn dialog_leaf_response_has_no_reply() {
        let toml = r#"
entity_type = "character"

[persona]
type = "standard"

[persona.dialog_tree]
text = "Welcome!"

[[persona.dialog_tree.responses]]
text = "Goodbye."
"#;
        let config: EntityConfig = toml::from_str(toml).unwrap();
        if let Some(PersonaConfig::Standard {
            dialog_tree: Some(tree),
        }) = &config.persona
        {
            assert!(tree.responses[0].reply.is_none());
        } else {
            panic!("expected Standard persona with dialog_tree");
        }
    }

    #[test]
    fn dialog_tree_multi_level_round_trip() {
        let toml = r#"
entity_type = "character"

[persona]
type = "standard"

[persona.dialog_tree]
text = "Welcome to the tavern!"

[[persona.dialog_tree.responses]]
text = "I'd like a room."

[persona.dialog_tree.responses.reply]
text = "That'll be 5 gold."

[[persona.dialog_tree.responses.reply.responses]]
text = "Here you go."

[persona.dialog_tree.responses.reply.responses.reply]
text = "Enjoy your stay!"

[[persona.dialog_tree.responses.reply.responses]]
text = "Never mind."
"#;
        let config: EntityConfig = toml::from_str(toml).unwrap();
        let serialized = toml::to_string(&config).unwrap();
        let config2: EntityConfig = toml::from_str(&serialized).unwrap();
        if let Some(PersonaConfig::Standard {
            dialog_tree: Some(tree),
        }) = &config2.persona
        {
            assert_eq!(tree.text, "Welcome to the tavern!");
            let reply = tree.responses[0].reply.as_ref().unwrap();
            assert_eq!(reply.text, "That'll be 5 gold.");
            assert_eq!(reply.responses.len(), 2);
            assert_eq!(
                reply.responses[0].reply.as_ref().unwrap().text,
                "Enjoy your stay!"
            );
            assert!(reply.responses[1].reply.is_none());
        } else {
            panic!("expected Standard persona with dialog_tree");
        }
    }

    #[test]
    fn object_config_round_trip() {
        let toml = r#"
entity_type = "object"
"#;
        let config: EntityConfig = toml::from_str(toml).unwrap();
        assert!(matches!(config.entity_type, EntityTypeConfig::Object));
        assert!(config.attributes.is_empty());
        assert!(config.entity_effects.is_empty());
    }

    #[test]
    fn load_entity_configs_returns_empty_when_no_entities_dir() {
        let tmp = TempDir::new().unwrap();
        let configs = load_entity_configs(tmp.path()).unwrap();
        assert!(configs.is_empty());
    }

    #[test]
    fn load_entity_configs_finds_toml_files() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "entities/innkeeper.toml",
            r#"entity_type = "character""#,
        );
        let configs = load_entity_configs(tmp.path()).unwrap();
        assert_eq!(configs.len(), 1);
        assert!(configs.contains_key("entities/innkeeper"));
    }

    #[test]
    fn load_entity_configs_uses_id_field_when_present() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "entities/myfile.toml",
            r#"
id = "custom_id"
entity_type = "character"
"#,
        );
        let configs = load_entity_configs(tmp.path()).unwrap();
        assert!(configs.contains_key("custom_id"));
    }
}
