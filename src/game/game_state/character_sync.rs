use std::collections::HashSet;

use sqlx::SqlitePool;

use crate::game::GameState;
use crate::game::entity::character::Character;
use crate::persistence::{PersistenceError, character_repo};

pub async fn sync(game_state: &GameState, pool: &SqlitePool) -> Result<(), PersistenceError> {
    let new_active_dungeons = compute_active_dungeons(game_state).await;

    if *game_state.active_dungeons.read().await == new_active_dungeons {
        return Ok(());
    }

    let mut incoming: Vec<Character> = Vec::new();
    for (world_id, dungeon_id) in &new_active_dungeons {
        incoming.extend(load_dungeon_characters(pool, world_id, dungeon_id).await?);
    }

    {
        let mut characters = game_state.active_characters.write().await;
        characters.retain(|_, c| c.config_id.is_none());
        for character in incoming {
            characters.insert(character.id, character);
        }
    }

    *game_state.active_dungeons.write().await = new_active_dungeons;
    Ok(())
}

async fn compute_active_dungeons(game_state: &GameState) -> HashSet<(String, String)> {
    let characters = game_state.active_characters.read().await;
    let players = game_state.active_players.read().await;
    players
        .values()
        .filter_map(|p| characters.get(&p.entity_id))
        .map(|c| (c.location.world_id.clone(), c.location.dungeon_id.clone()))
        .collect()
}

async fn load_dungeon_characters(
    pool: &SqlitePool,
    world_id: &str,
    dungeon_id: &str,
) -> Result<Vec<Character>, PersistenceError> {
    character_repo::find_config_characters_by_dungeon(pool, world_id, dungeon_id).await
}
