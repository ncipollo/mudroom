mod handlers;
mod message_relay;
mod ping_reaper;
mod router;
mod state;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::RwLock;

use crate::game::{self, GameState};
use crate::network::session::ServerSession;
use crate::persistence::Database;
use state::{AppState, ConnectedClient};

pub async fn start(
    server_session: ServerSession,
    game_state: GameState,
    db: Database,
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let connections: Arc<RwLock<HashMap<String, ConnectedClient>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let state = Arc::new(AppState {
        server_session,
        game_state: Arc::new(game_state),
        db,
        connections: connections.clone(),
        next_connection_seq: Arc::new(std::sync::atomic::AtomicU64::new(1)),
    });

    message_relay::spawn(
        state.game_state.message_tx.subscribe(),
        state.connections.clone(),
        state.game_state.clone(),
    );

    tokio::spawn(game::game_loop::run(
        state.game_state.clone(),
        state.db.clone(),
    ));

    tokio::spawn(ping_reaper::run_ping_reaper(
        connections,
        state.game_state.clone(),
    ));

    let router = router::build_router(state);
    let listener = TcpListener::bind("0.0.0.0:0").await?;
    let addr = listener.local_addr()?;

    tokio::spawn(async move {
        axum::serve(listener, router).await.ok();
    });

    Ok(addr)
}
