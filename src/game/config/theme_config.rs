use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeStyleConfig {
    #[serde(default)]
    pub fg: Option<String>,
    #[serde(default)]
    pub bg: Option<String>,
    #[serde(default)]
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub id: Option<String>,
    #[serde(default)]
    pub styles: HashMap<String, ThemeStyleConfig>,
}

pub fn load_themes(config_dir: &Path) -> Result<HashMap<String, ThemeConfig>, Box<dyn Error>> {
    let mut configs = HashMap::new();
    let themes_dir = config_dir.join("themes");
    if !themes_dir.exists() {
        return Ok(configs);
    }
    for entry in walkdir::WalkDir::new(&themes_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"))
    {
        let path = entry.path();
        let content = std::fs::read_to_string(path)?;
        let mut config: ThemeConfig = toml::from_str(&content)?;
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

/// Resolves a description's theme id against the loaded themes, falling back to the client's
/// standard (built-in default) theme — represented here as `None` — when the id is absent or
/// unknown. An unknown id is warned about since it likely indicates a mud author typo; an absent
/// id is the normal case and isn't logged.
pub fn resolve_theme_id(
    themes: &HashMap<String, ThemeConfig>,
    theme_id: Option<&str>,
) -> Option<String> {
    match theme_id {
        None => None,
        Some(id) if themes.contains_key(id) => Some(id.to_string()),
        Some(id) => {
            tracing::warn!(theme_id = %id, "unknown theme id, falling back to standard theme");
            None
        }
    }
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
    fn load_themes_returns_empty_when_no_themes_dir() {
        let tmp = TempDir::new().unwrap();
        let configs = load_themes(tmp.path()).unwrap();
        assert!(configs.is_empty());
    }

    #[test]
    fn load_themes_finds_toml_files() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "themes/eerie.toml",
            r#"
[styles.bold]
fg = "red"
modifiers = ["bold"]
"#,
        );
        let configs = load_themes(tmp.path()).unwrap();
        assert_eq!(configs.len(), 1);
        assert!(configs.contains_key("eerie"));
    }

    #[test]
    fn load_themes_uses_id_field_when_present() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "themes/myfile.toml",
            r#"
id = "custom_id"
"#,
        );
        let configs = load_themes(tmp.path()).unwrap();
        assert!(configs.contains_key("custom_id"));
        assert!(!configs.contains_key("myfile"));
    }

    #[test]
    fn theme_config_round_trip() {
        let toml = r#"
[styles.bold]
fg = "red"
modifiers = ["bold"]

[styles.emphasis]
fg = "magenta"
modifiers = ["italic"]

[styles.highlight]
fg = "cyan"
"#;
        let config: ThemeConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.styles.len(), 3);
        assert_eq!(config.styles["bold"].fg.as_deref(), Some("red"));
        assert_eq!(config.styles["bold"].modifiers, vec!["bold".to_string()]);
        assert_eq!(config.styles["highlight"].fg.as_deref(), Some("cyan"));
        assert!(config.styles["highlight"].modifiers.is_empty());
    }

    #[test]
    fn resolve_theme_id_returns_none_for_absent_id() {
        let themes = HashMap::new();
        assert_eq!(resolve_theme_id(&themes, None), None);
    }

    #[test]
    fn resolve_theme_id_returns_none_for_unknown_id() {
        let themes = HashMap::new();
        assert_eq!(resolve_theme_id(&themes, Some("nonexistent")), None);
    }

    #[test]
    fn resolve_theme_id_returns_id_when_known() {
        let mut themes = HashMap::new();
        themes.insert("eerie".to_string(), ThemeConfig::default());
        assert_eq!(
            resolve_theme_id(&themes, Some("eerie")),
            Some("eerie".to_string())
        );
    }
}
