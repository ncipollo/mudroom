use std::collections::HashMap;
use std::path::Path;

use thiserror::Error;

use crate::game::script::Script;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ScriptLookupError {
    #[error("script not found: {0}")]
    NotFound(String),
}

/// Resolves `relative_path` against `config_dir`'s `scripts/` folder, following the same
/// mud-root-relative convention as `config_path.rs` and the other config loaders. Accepts either
/// a path with an explicit extension (e.g. `abilities/damage_bonus.js`) or a bare name (e.g.
/// `abilities/damage_bonus`), assuming `.js` when none is given.
pub fn find_script(config_dir: &Path, relative_path: &str) -> Result<Script, ScriptLookupError> {
    let mut path = config_dir.join("scripts").join(relative_path);
    if path.extension().is_none() {
        path.set_extension("js");
    }
    if path.is_file() {
        Ok(Script::new(path))
    } else {
        Err(ScriptLookupError::NotFound(path.display().to_string()))
    }
}

/// Walks `config_dir/scripts` and loads every `.js` file found, keyed by its path relative to
/// the `scripts/` folder with the extension stripped (e.g. `abilities/damage_bonus`), mirroring
/// `build_ability_cache`'s id convention.
pub fn load_scripts(
    config_dir: &Path,
) -> Result<HashMap<String, Script>, Box<dyn std::error::Error>> {
    let mut scripts = HashMap::new();
    let scripts_dir = config_dir.join("scripts");
    if !scripts_dir.exists() {
        return Ok(scripts);
    }
    for entry in walkdir::WalkDir::new(&scripts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("js"))
    {
        let rel = entry.path().strip_prefix(&scripts_dir)?.with_extension("");
        let name = rel.to_string_lossy().to_string();
        let script = find_script(config_dir, &name)?;
        scripts.insert(name, script);
    }
    Ok(scripts)
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
    fn find_script_resolves_explicit_extension() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "scripts/abilities/damage_bonus.js", "// bonus");
        let script = find_script(tmp.path(), "abilities/damage_bonus.js").unwrap();
        assert!(script.path.ends_with("scripts/abilities/damage_bonus.js"));
    }

    #[test]
    fn find_script_resolves_bare_name() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "scripts/abilities/damage_bonus.js", "// bonus");
        let script = find_script(tmp.path(), "abilities/damage_bonus").unwrap();
        assert!(script.path.ends_with("scripts/abilities/damage_bonus.js"));
    }

    #[test]
    fn find_script_errors_when_missing() {
        let tmp = TempDir::new().unwrap();
        let result = find_script(tmp.path(), "abilities/missing");
        assert!(matches!(result, Err(ScriptLookupError::NotFound(_))));
    }

    #[test]
    fn load_scripts_returns_empty_when_no_scripts_dir() {
        let tmp = TempDir::new().unwrap();
        let scripts = load_scripts(tmp.path()).unwrap();
        assert!(scripts.is_empty());
    }

    #[test]
    fn load_scripts_finds_nested_js_files() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "scripts/abilities/damage_bonus.js", "// bonus");
        let scripts = load_scripts(tmp.path()).unwrap();
        assert_eq!(scripts.len(), 1);
        assert!(scripts.contains_key("abilities/damage_bonus"));
    }

    #[test]
    fn load_scripts_ignores_non_js_files() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "scripts/notes.txt", "not a script");
        let scripts = load_scripts(tmp.path()).unwrap();
        assert!(scripts.is_empty());
    }

    #[test]
    fn find_script_resolves_the_example_damage_bonus_script() {
        let script = find_script(Path::new("muds/basic"), "abilities/damage_bonus.js").unwrap();
        assert!(
            script
                .path
                .ends_with("muds/basic/scripts/abilities/damage_bonus.js")
        );
    }
}
