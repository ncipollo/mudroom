use sqlx::SqlitePool;
use std::error::Error;

use crate::game::config::{FactionConfig, ResourceConfig};
use crate::persistence::{faction_repo, resource_repo};

pub async fn load_factions_into_db(
    pool: &SqlitePool,
    faction_config: &FactionConfig,
) -> Result<(), Box<dyn Error>> {
    for faction in &faction_config.factions {
        faction_repo::upsert(pool, faction).await?;
    }
    Ok(())
}

pub async fn load_resources_into_db(
    pool: &SqlitePool,
    resource_config: &ResourceConfig,
) -> Result<(), Box<dyn Error>> {
    for resource in &resource_config.resources {
        resource_repo::upsert_definition(pool, resource).await?;
    }
    Ok(())
}
