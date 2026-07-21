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

const HEADER: &str = r#"mudroom instructions — authoring guidance for mud config files

Usage: mudroom instructions [topic]  (alias: mudroom info [topic])"#;

pub fn render() -> String {
    let mut result = HEADER.to_string();
    if TOPICS.is_empty() {
        result.push_str("\n\nNo topics are registered yet.");
    } else {
        let width = TOPICS.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
        result.push_str("\n\nTopics:\n");
        let topic_lines: Vec<String> = TOPICS
            .iter()
            .map(|(name, description)| {
                format!("  mudroom instructions {name:<width$}  — {description}")
            })
            .collect();
        result.push_str(&topic_lines.join("\n"));
    }
    result
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
