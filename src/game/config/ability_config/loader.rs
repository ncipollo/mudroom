use super::config::Ability;
use std::error::Error;
use std::path::Path;

pub fn load_ability(path: &Path) -> Result<Ability, Box<dyn Error>> {
    let content = std::fs::read_to_string(path)?;
    let ability: Ability = toml::from_str(&content)?;
    Ok(ability)
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
    fn load_ability_parses_toml() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "basic_attack.toml",
            r#"
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
        let ability = load_ability(&tmp.path().join("basic_attack.toml")).unwrap();
        assert_eq!(ability.name, "Basic Attack");
    }
}
