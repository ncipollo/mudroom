use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

use sqlx::SqlitePool;

use crate::game::component::Ability;
use crate::game::config::ability_config::load_ability;
use crate::persistence::ability_repo;

/// Upserts every ability in `ability_cache` into the database. This keeps the stored
/// ability rows in sync with the config files (e.g. action_text templates), independent
/// of whether a room's characters actually reference them.
pub async fn sync_abilities_into_db(
    pool: &SqlitePool,
    ability_cache: &HashMap<String, Ability>,
) -> Result<(), Box<dyn Error>> {
    for ability in ability_cache.values() {
        ability_repo::upsert(pool, ability).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::description::Description;
    use crate::persistence::ability_repo;
    use crate::persistence::database::Database;

    fn make_ability(action_text: Option<String>) -> Ability {
        Ability {
            id: "basic_attack".to_string(),
            name: "Basic Attack".to_string(),
            description: Description::default(),
            effects: vec![],
            costs: vec![],
            modifiers: vec![],
            engagement_types: vec![],
            role: crate::game::component::AbilityRole::Attack,
            targets: vec![],
            action_text,
        }
    }

    #[tokio::test]
    async fn sync_abilities_into_db_updates_stale_action_text() {
        let db = Database::connect_in_memory().await.unwrap();

        // Seed a stale row (as if written before the config changed).
        let stale = make_ability(Some("{{entity}} strikes {{target}}".to_string()));
        ability_repo::upsert(db.pool(), &stale).await.unwrap();

        // Config now carries the corrected template.
        let mut cache = HashMap::new();
        cache.insert(
            "basic_attack".to_string(),
            make_ability(Some("{{character}} strikes {{target}}".to_string())),
        );

        sync_abilities_into_db(db.pool(), &cache).await.unwrap();

        let row: Option<String> =
            sqlx::query_scalar("SELECT action_text FROM abilities WHERE id = ?")
                .bind("basic_attack")
                .fetch_optional(db.pool())
                .await
                .unwrap();
        assert_eq!(row.as_deref(), Some("{{character}} strikes {{target}}"));
    }
}

pub fn build_ability_cache(config_dir: &Path) -> Result<HashMap<String, Ability>, Box<dyn Error>> {
    let mut cache = HashMap::new();
    let abilities_dir = config_dir.join("abilities");
    if !abilities_dir.exists() {
        return Ok(cache);
    }
    for entry in walkdir::WalkDir::new(&abilities_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"))
    {
        let path = entry.path();
        let mut ability = load_ability(path)?;
        let rel = path.strip_prefix(&abilities_dir)?.with_extension("");
        ability.id = rel.to_string_lossy().to_string();
        cache.insert(ability.id.clone(), ability);
    }
    Ok(cache)
}
