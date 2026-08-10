use super::config::ItemDefinition;
use std::error::Error;
use std::path::Path;

pub fn load_item(path: &Path) -> Result<ItemDefinition, Box<dyn Error>> {
    let content = std::fs::read_to_string(path)?;
    let item: ItemDefinition = toml::from_str(&content)?;
    Ok(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn load_item_parses_toml() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("health_tonic.toml");
        fs::write(
            &path,
            r#"
name = "Health Tonic"
description = "A restorative brew."
use_type = "used"
item_type = "medicine"
"#,
        )
        .unwrap();
        let item = load_item(&path).unwrap();
        assert_eq!(item.name, "Health Tonic");
    }
}
