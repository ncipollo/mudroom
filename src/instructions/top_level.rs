const TOPICS: &[(&str, &str)] = &[
    (
        "mud-config",
        "Mud directory layout and mud.toml configuration",
    ),
    (
        "abilities",
        "Ability config file (abilities/*.toml) reference",
    ),
    (
        "attributes",
        "Attribute definitions (attributes.toml) reference",
    ),
    ("classes", "Class config file (classes/*.toml) reference"),
    ("entities", "Entity config file (entities/*.toml) reference"),
    (
        "maps",
        "Map and room config file (maps/<world>/<dungeon>/<room>.toml) reference",
    ),
];

pub fn render() -> String {
    let mut lines = vec![
        "mudroom instructions — authoring guidance for mud config files".to_string(),
        String::new(),
        "Usage: mudroom instructions [topic]  (alias: mudroom info [topic])".to_string(),
    ];
    if TOPICS.is_empty() {
        lines.push(String::new());
        lines.push("No topics are registered yet.".to_string());
    } else {
        let width = TOPICS.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
        lines.push(String::new());
        lines.push("Topics:".to_string());
        for (name, description) in TOPICS {
            lines.push(format!(
                "  mudroom instructions {name:<width$}  — {description}"
            ));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_usage_header() {
        let text = render();
        assert!(text.contains("mudroom instructions [topic]"));
    }

    #[test]
    fn render_lists_registered_topics() {
        let text = render();
        assert!(text.contains("Topics:"));
        assert!(text.contains("mud-config"));
        assert!(text.contains("maps"));
    }
}
