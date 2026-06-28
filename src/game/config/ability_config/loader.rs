use super::config::Ability;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

pub fn load_ability(path: &Path) -> Result<Ability, Box<dyn Error>> {
    let content = std::fs::read_to_string(path)?;
    let ability: Ability = toml::from_str(&content)?;
    Ok(ability)
}

pub fn load_abilities(config_dir: &Path) -> Result<HashMap<String, Ability>, Box<dyn Error>> {
    let mut abilities = HashMap::new();
    let abilities_dir = config_dir.join("abilities");
    if !abilities_dir.exists() {
        return Ok(abilities);
    }
    for entry in walkdir::WalkDir::new(&abilities_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"))
    {
        let path = entry.path();
        let ability = load_ability(path)?;
        abilities.insert(ability.id.clone(), ability);
    }
    Ok(abilities)
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
    fn load_abilities_returns_empty_when_no_abilities_dir() {
        let tmp = TempDir::new().unwrap();
        let abilities = load_abilities(tmp.path()).unwrap();
        assert!(abilities.is_empty());
    }

    #[test]
    fn load_abilities_finds_toml_files() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "abilities/basic_attack.toml",
            r#"
id = "basic_attack"
name = "Basic Attack"
engagement_types = ["battle"]
costs = []
role = "attack"

[[effects]]
name = "physical_damage"
trigger_info = { type = "once" }
effect_type = { type = "attribute_update", attribute_id = "hp", value = -8 }
"#,
        );
        let abilities = load_abilities(tmp.path()).unwrap();
        assert_eq!(abilities.len(), 1);
        assert!(abilities.contains_key("basic_attack"));
        assert_eq!(abilities["basic_attack"].name, "Basic Attack");
    }
}
