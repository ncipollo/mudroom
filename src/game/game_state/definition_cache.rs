use sqlx::SqlitePool;

use crate::game::GameState;
use crate::persistence::{PersistenceError, ability_repo, item_repo};

pub async fn refresh(game_state: &GameState, pool: &SqlitePool) -> Result<(), PersistenceError> {
    let abilities = ability_repo::find_all(pool).await?;
    let item_definitions = item_repo::find_all_definitions(pool).await?;

    *game_state.abilities.write().await =
        abilities.into_iter().map(|a| (a.id.clone(), a)).collect();
    *game_state.item_definitions.write().await = item_definitions
        .into_iter()
        .map(|d| (d.id.clone(), d))
        .collect();

    Ok(())
}
