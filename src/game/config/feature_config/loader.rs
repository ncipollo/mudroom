use std::error::Error;
use std::path::Path;

use crate::game::map::universe::room_feature::RoomFeature;

/// Parses a single feature definition from a TOML file and validates that its
/// `default_state` and every `interact_next_state` name a real state.
pub fn load_feature(path: &Path) -> Result<RoomFeature, Box<dyn Error>> {
    let content = std::fs::read_to_string(path)?;
    let feature: RoomFeature = toml::from_str(&content)?;
    feature.validate()?;
    Ok(feature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_feature(dir: &TempDir, name: &str, toml: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, toml).unwrap();
        path
    }

    #[test]
    fn load_feature_parses_toml() {
        let tmp = TempDir::new().unwrap();
        let path = write_feature(
            &tmp,
            "chest.toml",
            r#"
name = "Oak Chest"
default_state = "closed"

[states.closed]
description = "A closed chest."
interact_next_state = "open"

[states.open]
description = "An open chest."
items = ["medicine"]
"#,
        );
        let feature = load_feature(&path).unwrap();
        assert_eq!(feature.name, "Oak Chest");
        assert_eq!(feature.states.len(), 2);
    }

    #[test]
    fn load_feature_rejects_unknown_default_state() {
        let tmp = TempDir::new().unwrap();
        let path = write_feature(
            &tmp,
            "chest.toml",
            r#"
name = "Oak Chest"
default_state = "missing"

[states.closed]
description = "A closed chest."
"#,
        );
        assert!(load_feature(&path).is_err());
    }

    #[test]
    fn load_feature_parses_the_example_chest_feature() {
        let feature = load_feature(Path::new("muds/basic/features/chest.toml")).unwrap();
        let open = feature.states.get("open").unwrap();
        assert_eq!(open.items, vec!["medicine".to_string()]);
    }

    #[test]
    fn load_feature_rejects_dangling_interact_next_state() {
        let tmp = TempDir::new().unwrap();
        let path = write_feature(
            &tmp,
            "chest.toml",
            r#"
name = "Oak Chest"
default_state = "closed"

[states.closed]
description = "A closed chest."
interact_next_state = "missing"
"#,
        );
        assert!(load_feature(&path).is_err());
    }
}
