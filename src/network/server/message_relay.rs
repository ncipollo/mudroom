use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::sync::broadcast;

use crate::game::GameState;
use crate::game::messaging::{
    BattleStartedMessage, BattleUpdateMessage, InventoryOpenedMessage, Message, PlayerMessage,
    StreamingState,
};
use crate::network::event::{
    BattleSnapshot, InventoryItemInfo, InventorySlotInfo, NetworkEvent, ParticipantInfo,
};

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
            let (player_id, network_event) = match pm.message {
                Message::Complete { content, theme } => (
                    pm.player_id,
                    NetworkEvent::Message {
                        player_id: pm.player_id,
                        content,
                        theme,
                    },
                ),
                Message::Streaming { chunk, state } => {
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
                Message::ConversationStarted { kind, options } => (
                    pm.player_id,
                    NetworkEvent::ConversationStarted { kind, options },
                ),
                Message::ConversationEnded => (pm.player_id, NetworkEvent::ConversationEnded),
                Message::BattleStarted(data) => (
                    pm.player_id,
                    NetworkEvent::BattleStarted {
                        engagement_id: data.engagement_id,
                        snapshot: battle_snapshot_from_message(*data),
                    },
                ),
                Message::BattleUpdate(data) => {
                    let messages = data.messages.clone();
                    let engagement_id = data.engagement_id;
                    (
                        pm.player_id,
                        NetworkEvent::BattleUpdate {
                            engagement_id,
                            snapshot: battle_update_snapshot(*data),
                            messages,
                        },
                    )
                }
                Message::BattleEnded { engagement_id } => {
                    (pm.player_id, NetworkEvent::BattleEnded { engagement_id })
                }
                Message::InventoryOpened(data) => (pm.player_id, inventory_opened_event(*data)),
                Message::PlayerStatsUpdated {
                    hp_current,
                    hp_max,
                    mp_current,
                    mp_max,
                } => (
                    pm.player_id,
                    NetworkEvent::PlayerStatsUpdated {
                        hp_current,
                        hp_max,
                        mp_current,
                        mp_max,
                    },
                ),
            };

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

fn inventory_opened_event(data: InventoryOpenedMessage) -> NetworkEvent {
    let slots = data
        .slots
        .into_iter()
        .map(|s| InventorySlotInfo {
            slot_name: s.slot_name,
            item_types: s.item_types,
            equipped: s.equipped.map(inventory_item_info),
        })
        .collect();
    let bag = data.bag.into_iter().map(inventory_item_info).collect();
    NetworkEvent::InventoryOpened {
        slots,
        bag,
        bag_size: data.bag_size,
    }
}

fn inventory_item_info(item: crate::game::messaging::InventoryItemInfo) -> InventoryItemInfo {
    InventoryItemInfo {
        item_id: item.item_id,
        name: item.name,
        item_type: item.item_type,
        description: item.description,
        usable: item.usable,
        equippable: item.equippable,
    }
}

fn battle_update_snapshot(data: BattleUpdateMessage) -> BattleSnapshot {
    let participants = data
        .participants
        .into_iter()
        .map(|(faction, infos)| {
            let converted = infos
                .into_iter()
                .map(|p| ParticipantInfo {
                    id: p.id,
                    name: p.name,
                    hp_current: p.hp_current,
                    hp_max: p.hp_max,
                })
                .collect();
            (faction, converted)
        })
        .collect();
    BattleSnapshot {
        factions: data.factions,
        participants,
        phase: data.phase,
        turn_order: vec![],
        countdown_secs: data.countdown_secs,
        max_turn_secs: data.max_turn_secs,
        available_abilities: data.available_abilities,
    }
}

fn battle_snapshot_from_message(data: BattleStartedMessage) -> BattleSnapshot {
    let participants = data
        .participants
        .into_iter()
        .map(|(faction, infos)| {
            let converted = infos
                .into_iter()
                .map(|p| ParticipantInfo {
                    id: p.id,
                    name: p.name,
                    hp_current: p.hp_current,
                    hp_max: p.hp_max,
                })
                .collect();
            (faction, converted)
        })
        .collect();
    BattleSnapshot {
        factions: data.factions,
        participants,
        phase: data.phase,
        turn_order: data.turn_order,
        countdown_secs: data.countdown_secs,
        max_turn_secs: data.max_turn_secs,
        available_abilities: data.available_abilities,
    }
}
