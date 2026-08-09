mod ability_cache;
mod character_sync;
mod definition_sync;
mod universe_sync;

pub use ability_cache::{build_ability_cache as load_abilities, sync_abilities_into_db};
pub use character_sync::load_characters_into_db;
pub use definition_sync::{load_factions_into_db, load_resources_into_db};
pub use universe_sync::{load_map_into_db, should_auto_load};

use sqlx::SqlitePool;
use std::error::Error;
use std::path::Path;

use crate::game::config::character_config::load_character_configs;
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
        let character_configs = load_character_configs(config_dir)?;
        let ability_cache = ability_cache::build_ability_cache(config_dir)?;
        ability_cache::sync_abilities_into_db(pool, &ability_cache).await?;
        load_characters_into_db(pool, &universe, &character_configs, &ability_cache).await?;
    }
    Ok(())
}
