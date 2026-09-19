use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

use crate::game::config::feature_config::loader::load_feature;
use crate::game::map::universe::room_feature::RoomFeature;

/// Walks `config_dir/features` and loads every feature definition found there,
/// deriving each feature's id from its path relative to that directory (e.g.
/// `features/chest.toml` -> `chest`). Returns an empty map if the directory
/// doesn't exist.
pub fn build_feature_map(
    config_dir: &Path,
) -> Result<HashMap<String, RoomFeature>, Box<dyn Error>> {
    let mut features = HashMap::new();
    let features_dir = config_dir.join("features");
    if !features_dir.exists() {
        return Ok(features);
    }
    for entry in walkdir::WalkDir::new(&features_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"))
    {
        let path = entry.path();
        let mut feature = load_feature(path)?;
        let rel = path.strip_prefix(&features_dir)?.with_extension("");
        feature.id = rel.to_string_lossy().to_string();
        features.insert(feature.id.clone(), feature);
    }
    Ok(features)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_feature(dir: &Path, relative: &str, toml: &str) {
        let path = dir.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, toml).unwrap();
    }

    const CHEST_TOML: &str = r#"
name = "Oak Chest"
default_state = "closed"

[states.closed]
description = "A closed chest."
"#;

    #[test]
    fn build_feature_map_returns_empty_for_missing_dir() {
        let dir = std::env::temp_dir().join("mudroom_feature_map_test_missing");
        let features = build_feature_map(&dir).unwrap();
        assert!(features.is_empty());
    }

    #[test]
    fn build_feature_map_derives_id_from_file_name() {
        let tmp = TempDir::new().unwrap();
        write_feature(&tmp.path().join("features"), "chest.toml", CHEST_TOML);

        let features = build_feature_map(tmp.path()).unwrap();

        assert_eq!(features.len(), 1);
        assert_eq!(features["chest"].id, "chest");
    }

    #[test]
    fn build_feature_map_derives_id_from_nested_path() {
        let tmp = TempDir::new().unwrap();
        write_feature(
            &tmp.path().join("features"),
            "dungeon/chest.toml",
            CHEST_TOML,
        );

        let features = build_feature_map(tmp.path()).unwrap();

        assert!(features.contains_key("dungeon/chest"));
    }

    #[test]
    fn build_feature_map_propagates_load_errors() {
        let tmp = TempDir::new().unwrap();
        write_feature(
            &tmp.path().join("features"),
            "broken.toml",
            r#"
name = "Broken"
default_state = "missing"

[states.closed]
description = "A closed chest."
"#,
        );

        assert!(build_feature_map(tmp.path()).is_err());
    }
}
