mod handlers;
mod ping_reaper;
mod router;
mod state;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::RwLock;

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
    let connections: Arc<RwLock<HashMap<String, ConnectedClient>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let state = Arc::new(AppState {
        server_session,
        game_state: Arc::new(game_state),
        db,
        connections: connections.clone(),
        config_path,
    });

    let mut msg_rx = state.game_state.message_tx.subscribe();
    let connections_relay = state.connections.clone();
    let game_state_relay = state.game_state.clone();
    tokio::spawn(async move {
        while let Ok(pm) = msg_rx.recv().await {
            let (player_id, network_event) = match pm.message {
                game::messaging::Message::Complete(content) => (
                    pm.player_id,
                    NetworkEvent::Message {
                        player_id: pm.player_id,
                        content,
                    },
                ),
                game::messaging::Message::Streaming { chunk, state } => {
                    let is_final = matches!(state, game::messaging::StreamingState::Complete);
                    (
                        pm.player_id,
                        NetworkEvent::MessageChunk {
                            player_id: pm.player_id,
                            chunk,
                            is_final,
                        },
                    )
                }
            };
            let players = game_state_relay.active_players.read().await;
            let client_id = players
                .iter()
                .find(|(_, p)| p.id == player_id)
                .map(|(cid, _): (&String, _)| cid.clone());
            drop(players);
            if let Some(cid) = client_id {
                let conns = connections_relay.read().await;
                if let Some(client) = conns.get(&cid) {
                    let _ = client.personal_tx.send(network_event).await;
                }
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

    tokio::spawn(ping_reaper::run_ping_reaper(connections));

    Ok(addr)
}
