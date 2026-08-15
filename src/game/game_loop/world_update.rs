use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::game::GameState;
use crate::game::sync_universe_config;
use crate::persistence::Database;

pub async fn process(game_state: &Arc<GameState>, db: &Database, tick: u64) {
    process_reload(game_state, db).await;
    process_world_update(game_state, tick);
}

async fn process_reload(game_state: &Arc<GameState>, db: &Database) {
    if !game_state.reload_pending.swap(false, Ordering::AcqRel) {
        return;
    }
    if !sync_universe_and_log(game_state, db).await {
        return;
    }
    refresh_definition_caches(game_state, db).await;
}

/// Returns `true` on success. Kept as a standalone `async fn` so the non-`Send`
/// `Box<dyn Error>` from `sync_universe_config` is fully resolved before returning, rather than
/// living across the subsequent `refresh_definition_caches` await point in `process_reload`.
async fn sync_universe_and_log(game_state: &Arc<GameState>, db: &Database) -> bool {
    let result = sync_universe_config(
        db.pool(),
        game_state.config_path.as_deref(),
        &game_state.faction_config,
        &game_state.resource_config,
    )
    .await;
    match result {
        Ok(()) => true,
        Err(e) => {
            log_reload_error(e);
            false
        }
    }
}

async fn refresh_definition_caches(game_state: &Arc<GameState>, db: &Database) {
    match game_state.refresh_definition_caches(db.pool()).await {
        Ok(()) => log_reload_success(),
        Err(e) => log_reload_error(e),
    }
}

fn log_reload_success() {
    tracing::info!("Maps reloaded");
}

fn log_reload_error(e: impl std::fmt::Display) {
    tracing::error!("Failed to reload maps: {e}");
}

fn process_world_update(game_state: &GameState, tick: u64) {
    let tick_rate = game_state.mud_config.game_loop.tick_rate_ms;
    let world_update_ticks = (game_state.mud_config.game_loop.world_update_ms / tick_rate).max(1);
    if tick.is_multiple_of(world_update_ticks) {
        log_world_update(tick);
    }
}

fn log_world_update(tick: u64) {
    tracing::info!("Processing world update tick={tick}");
}
