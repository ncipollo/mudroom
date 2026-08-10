use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

use sqlx::SqlitePool;

use crate::game::component::ItemDefinition;
use crate::persistence::item_repo;

fn load_item(path: &Path) -> Result<ItemDefinition, Box<dyn Error>> {
    let content = std::fs::read_to_string(path)?;
    let item: ItemDefinition = toml::from_str(&content)?;
    Ok(item)
}

pub fn build_item_cache(
    config_dir: &Path,
) -> Result<HashMap<String, ItemDefinition>, Box<dyn Error>> {
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

/// Upserts every item definition in `item_cache` into the database, keeping stored rows
/// in sync with the config files.
pub async fn sync_items_into_db(
    pool: &SqlitePool,
    item_cache: &HashMap<String, ItemDefinition>,
) -> Result<(), Box<dyn Error>> {
    for item in item_cache.values() {
        item_repo::upsert_definition(pool, item).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::ItemUseType;
    use crate::persistence::database::Database;

    fn make_item() -> ItemDefinition {
        ItemDefinition {
            id: "health_tonic".to_string(),
            name: "Health Tonic".to_string(),
            description: crate::game::component::description::Description::default(),
            use_type: ItemUseType::Used,
            item_type: "consumable".to_string(),
            attribute_bonuses: vec![],
            use_effects: vec![],
            equipped_abilities: vec![],
        }
    }

    #[tokio::test]
    async fn sync_items_into_db_updates_stale_name() {
        let db = Database::connect_in_memory().await.unwrap();

        let stale = make_item();
        item_repo::upsert_definition(db.pool(), &stale)
            .await
            .unwrap();

        let mut cache = HashMap::new();
        let mut updated = make_item();
        updated.name = "Greater Health Tonic".to_string();
        cache.insert("health_tonic".to_string(), updated);

        sync_items_into_db(db.pool(), &cache).await.unwrap();

        let row: Option<String> =
            sqlx::query_scalar("SELECT name FROM item_definitions WHERE id = ?")
                .bind("health_tonic")
                .fetch_optional(db.pool())
                .await
                .unwrap();
        assert_eq!(row.as_deref(), Some("Greater Health Tonic"));
    }

    #[test]
    fn build_item_cache_returns_empty_for_missing_dir() {
        let dir = std::env::temp_dir().join("mudroom_item_cache_test_missing");
        let cache = build_item_cache(&dir).unwrap();
        assert!(cache.is_empty());
    }
}
