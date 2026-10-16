pub mod activations;
pub mod attributes;
pub mod effects;
pub mod interactions;
pub mod world_update;

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, interval};

use crate::game::GameState;
use crate::game::engagement;
use crate::persistence::Database;

/// Runs the real-time ticker, also answering on-demand tick requests (from
/// `/debug/tick`) in between — both paths go through [`run_once`] on this same thread, since
/// `run_once` can hold a non-`Send` Boa `Context` across an await and can't be called from an
/// arbitrary (e.g. axum handler) task. See `network::server::game_loop_thread`.
pub async fn run(
    game_state: Arc<GameState>,
    db: Database,
    mut debug_tick_rx: mpsc::Receiver<oneshot::Sender<u64>>,
) {
    let tick_rate = game_state.mud_config.game_loop.tick_rate_ms;
    let mut ticker = interval(Duration::from_millis(tick_rate));

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                run_once(&game_state, &db).await;
            }
            Some(reply) = debug_tick_rx.recv() => {
                let tick = run_once(&game_state, &db).await;
                let _ = reply.send(tick);
            }
        }
    }
}

/// Runs exactly one tick of the game loop's processing sequence, out of band from the
/// real-time ticker. Shared by `run` and the debug tick-advance endpoint so both paths stay
/// on the same tick counter (see `GameState::next_tick`).
pub async fn run_once(game_state: &Arc<GameState>, db: &Database) -> u64 {
    let tick = game_state.next_tick();

    activations::process(game_state, db).await;
    interactions::process(game_state, db, tick).await;
    engagement::process(game_state, tick).await;
    effects::process(game_state, tick).await;
    attributes::process(game_state, tick).await;
    world_update::process(game_state, db, tick).await;

    tick
}
