use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

use sqlx::SqlitePool;

use crate::game::component::ItemDefinition;
use crate::game::config::item_config::load_item;
use crate::persistence::item_repo;

/// Upserts every item definition found under `config_dir/items` into the database, keeping
/// stored rows in sync with the config files.
pub async fn sync_items_into_db(
    pool: &SqlitePool,
    config_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let item_map = build_item_map(config_dir)?;
    for item in item_map.values() {
        item_repo::upsert_definition(pool, item).await?;
    }
    Ok(())
}

fn build_item_map(config_dir: &Path) -> Result<HashMap<String, ItemDefinition>, Box<dyn Error>> {
    let mut cache = HashMap::new();
    let items_dir = config_dir.join("items");
    if !items_dir.exists() {
        return Ok(cache);
    }
    for entry in walkdir::WalkDir::new(&items_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"))
    {
        let path = entry.path();
        let mut item = load_item(path)?;
        let rel = path.strip_prefix(&items_dir)?.with_extension("");
        item.id = rel.to_string_lossy().to_string();
        cache.insert(item.id.clone(), item);
    }
    Ok(cache)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::{EquippedBonuses, ItemUseType};
    use crate::persistence::database::Database;
    use std::fs;
    use tempfile::TempDir;

    fn make_item() -> ItemDefinition {
        ItemDefinition {
            id: "health_tonic".to_string(),
            name: "Health Tonic".to_string(),
            description: crate::game::component::description::Description::default(),
            use_type: ItemUseType::Used,
            item_type: "medicine".to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects: vec![],
            alternate_names: vec![],
        }
    }

    #[tokio::test]
    async fn sync_items_into_db_updates_stale_name() {
        let db = Database::connect_in_memory().await.unwrap();

        let stale = make_item();
        item_repo::upsert_definition(db.pool(), &stale)
            .await
            .unwrap();

        let tmp = TempDir::new().unwrap();
        let items_dir = tmp.path().join("items");
        fs::create_dir_all(&items_dir).unwrap();
        fs::write(
            items_dir.join("health_tonic.toml"),
            r#"
name = "Greater Health Tonic"
use_type = "used"
item_type = "medicine"
"#,
        )
        .unwrap();

        sync_items_into_db(db.pool(), tmp.path()).await.unwrap();

        let row: Option<String> =
            sqlx::query_scalar("SELECT name FROM item_definitions WHERE id = ?")
                .bind("health_tonic")
                .fetch_optional(db.pool())
                .await
                .unwrap();
        assert_eq!(row.as_deref(), Some("Greater Health Tonic"));
    }

    #[test]
    fn build_item_map_returns_empty_for_missing_dir() {
        let dir = std::env::temp_dir().join("mudroom_item_syncer_test_missing");
        let cache = build_item_map(&dir).unwrap();
        assert!(cache.is_empty());
    }
}
