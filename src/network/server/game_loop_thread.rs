use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::{mpsc, oneshot};
use tokio::task::LocalSet;

use crate::game::{self, GameState};
use crate::persistence::Database;

/// Polls the game loop via a `LocalSet` on its own OS thread instead of `tokio::spawn`, since
/// script execution holds a non-`Send` Boa `Context` across a real `.await`. The app stays
/// multi-threaded otherwise: this thread blocks on the shared `Handle`, so anything the loop
/// awaits outside the `LocalSet` still runs on the normal worker pool. Returns a sender for
/// on-demand tick requests (`/debug/tick`), which must run on this same thread too.
pub fn spawn(
    game_state: Arc<GameState>,
    db: Database,
) -> std::io::Result<mpsc::Sender<oneshot::Sender<u64>>> {
    let handle = Handle::current();
    let (debug_tick_tx, debug_tick_rx) = mpsc::channel(1);
    std::thread::Builder::new()
        .name("game-loop".to_string())
        .spawn(move || {
            let local = LocalSet::new();
            handle.block_on(local.run_until(game::game_loop::run(game_state, db, debug_tick_rx)));
        })?;
    Ok(debug_tick_tx)
}
