mod ability_cache;
mod definition_sync;
mod entity_sync;
mod universe_sync;

pub use definition_sync::{load_factions_into_db, load_resources_into_db};
pub use entity_sync::load_entities_into_db;
pub use universe_sync::{load_map_into_db, should_auto_load};

use sqlx::SqlitePool;
use std::error::Error;
use std::path::Path;

use crate::game::config::entity_config::load_entity_configs;
use crate::game::config::map_config::load_map;
use crate::game::config::{FactionConfig, ResourceConfig};

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
        let ability_cache = ability_cache::build_ability_cache(config_dir)?;
        load_entities_into_db(pool, &universe, &entity_configs, &ability_cache).await?;
    }
    Ok(())
}
