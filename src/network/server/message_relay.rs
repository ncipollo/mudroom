use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::sync::broadcast;

use crate::game::GameState;
use crate::game::messaging::{Message, PlayerMessage, StreamingState};
use crate::network::event::NetworkEvent;

use super::state::ConnectedClient;

/// Spawns the message relay task.
///
/// Subscribes to the game's broadcast channel and forwards each PlayerMessage
/// to the correct SSE client. Complete messages become NetworkEvent::Message;
/// streaming chunks become NetworkEvent::MessageChunk. The player's client_id
/// is resolved via active_players to find the right SSE channel.
pub fn spawn(
    mut msg_rx: broadcast::Receiver<PlayerMessage>,
    connections: Arc<RwLock<HashMap<String, ConnectedClient>>>,
    game_state: Arc<GameState>,
) {
    tokio::spawn(async move {
        while let Ok(pm) = msg_rx.recv().await {
            // Map the game message to the appropriate network event.
            let (player_id, network_event) = match pm.message {
                Message::Complete(content) => (
                    pm.player_id,
                    NetworkEvent::Message {
                        player_id: pm.player_id,
                        content,
                    },
                ),
                Message::Streaming { chunk, state } => {
                    // is_final signals the client that the streaming message is complete.
                    let is_final = matches!(state, StreamingState::Complete);
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
            // Look up the client_id for this player to find their SSE channel.
            let players = game_state.active_players.read().await;
            let client_id = players
                .iter()
                .find(|(_, p)| p.id == player_id)
                .map(|(cid, _): (&String, _)| cid.clone());
            drop(players);
            if let Some(cid) = client_id {
                let conns = connections.read().await;
                if let Some(client) = conns.get(&cid) {
                    let _ = client.personal_tx.send(network_event).await;
                }
            }
        }
    });
}
