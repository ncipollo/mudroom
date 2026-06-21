use std::path::Path;

pub fn entity_name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_name_uses_file_stem() {
        assert_eq!(
            entity_name_from_path(Path::new("entities/innkeeper.toml")),
            "innkeeper"
        );
    }

    #[test]
    fn entity_name_uses_file_stem_without_directory() {
        assert_eq!(
            entity_name_from_path(Path::new("innkeeper.toml")),
            "innkeeper"
        );
    }

    #[test]
    fn entity_name_falls_back_to_unknown_for_bare_path() {
        assert_eq!(entity_name_from_path(Path::new("")), "unknown");
    }
}
