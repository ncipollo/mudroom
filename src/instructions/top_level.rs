const TOPICS: &[(&str, &str)] = &[];

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
    fn render_notes_no_topics_when_empty() {
        let text = render();
        assert!(text.contains("No topics are registered yet."));
    }
}
