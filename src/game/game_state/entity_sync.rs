use std::collections::HashSet;

use sqlx::SqlitePool;

use crate::game::GameState;
use crate::game::entity::Entity;
use crate::persistence::{PersistenceError, entity_repo};

pub async fn sync(game_state: &GameState, pool: &SqlitePool) -> Result<(), PersistenceError> {
    let new_active_dungeons = compute_active_dungeons(game_state).await;

    if *game_state.active_dungeons.read().await == new_active_dungeons {
        return Ok(());
    }

    let mut incoming: Vec<Entity> = Vec::new();
    for (world_id, dungeon_id) in &new_active_dungeons {
        incoming.extend(load_dungeon_entities(pool, world_id, dungeon_id).await?);
    }

    {
        let mut entities = game_state.active_entities.write().await;
        entities.retain(|_, e| e.config_id.is_none());
        for entity in incoming {
            entities.insert(entity.id, entity);
        }
    }

    *game_state.active_dungeons.write().await = new_active_dungeons;
    Ok(())
}

async fn compute_active_dungeons(game_state: &GameState) -> HashSet<(String, String)> {
    let entities = game_state.active_entities.read().await;
    let players = game_state.active_players.read().await;
    players
        .values()
        .filter_map(|p| entities.get(&p.entity_id))
        .map(|e| (e.location.world_id.clone(), e.location.dungeon_id.clone()))
        .collect()
}

async fn load_dungeon_entities(
    pool: &SqlitePool,
    world_id: &str,
    dungeon_id: &str,
) -> Result<Vec<Entity>, PersistenceError> {
    entity_repo::find_config_entities_by_dungeon(pool, world_id, dungeon_id).await
}
