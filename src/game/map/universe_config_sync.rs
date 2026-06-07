use sqlx::SqlitePool;
use std::error::Error;
use std::path::Path;

use crate::game::config::{
    FactionConfig, ResourceConfig, load_entities_into_db, load_entity_configs,
    load_factions_into_db, load_map, load_map_into_db, load_resources_into_db,
};

pub async fn sync_universe_config(
    pool: &SqlitePool,
    config_path: Option<&Path>,
    faction_config: &FactionConfig,
    resource_config: &ResourceConfig,
) -> Result<(), Box<dyn Error>> {
    let universe = load_map(config_path)?;
    load_map_into_db(pool, &universe).await?;
    load_factions_into_db(pool, faction_config).await?;
    load_resources_into_db(pool, resource_config).await?;
    if let Some(config_dir) = config_path {
        let entity_configs = load_entity_configs(config_dir)?;
        load_entities_into_db(pool, &universe, &entity_configs).await?;
    }
    Ok(())
}
