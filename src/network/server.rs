mod handlers;
mod ping_reaper;
mod router;
mod state;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::{RwLock, broadcast};

use crate::game::{self, GameState};
use crate::network::event::NetworkEvent;
use crate::persistence::Database;
use crate::session::ServerSession;
use state::{AppState, ConnectedClient};

pub async fn start(
    server_session: ServerSession,
    game_state: GameState,
    db: Database,
    config_path: Option<PathBuf>,
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let (tx, _) = broadcast::channel::<NetworkEvent>(64);
    let connections: Arc<RwLock<HashMap<String, ConnectedClient>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let state = Arc::new(AppState {
        server_session,
        game_state: Arc::new(game_state),
        db,
        tx: tx.clone(),
        connections: connections.clone(),
        config_path,
    });

    let mut msg_rx = state.game_state.message_tx.subscribe();
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        while let Ok(pm) = msg_rx.recv().await {
            if let game::messaging::Message::Complete(content) = pm.message {
                let _ = tx_clone.send(NetworkEvent::Message {
                    player_id: pm.player_id,
                    content,
                });
            }
        }
    });

    tokio::spawn(game::game_loop::run(
        state.game_state.clone(),
        state.db.clone(),
    ));

    let router = router::build_router(state);
    let listener = TcpListener::bind("0.0.0.0:0").await?;
    let addr = listener.local_addr()?;

    tokio::spawn(async move {
        axum::serve(listener, router).await.ok();
    });

    tokio::spawn(ping_reaper::run_ping_reaper(connections, tx));

    Ok(addr)
}
