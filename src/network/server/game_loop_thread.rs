use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::task::LocalSet;

use crate::game::{self, GameState};
use crate::persistence::Database;

/// Polls the game loop via a `LocalSet` on its own OS thread instead of `tokio::spawn`, since
/// script execution needs to hold a non-`Send` Boa `Context` across a real `.await` and
/// `tokio::spawn` requires `Send`. The app stays multi-threaded otherwise: this thread blocks
/// on the same shared `Handle`, so anything the loop awaits outside the `LocalSet` still runs
/// on the normal worker pool.
pub fn spawn(game_state: Arc<GameState>, db: Database) -> std::io::Result<()> {
    let handle = Handle::current();
    std::thread::Builder::new()
        .name("game-loop".to_string())
        .spawn(move || {
            let local = LocalSet::new();
            handle.block_on(local.run_until(game::game_loop::run(game_state, db)));
        })?;
    Ok(())
}
