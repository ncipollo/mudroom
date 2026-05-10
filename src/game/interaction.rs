pub mod conversation;
pub mod help;
pub mod look;
pub mod movement;

use std::sync::Arc;

use tracing;

use crate::game::component::interaction::Movement;
use crate::game::player::Player;
use crate::game::{GameState, Interaction};
use crate::persistence::Database;

pub async fn process(game_state: &Arc<GameState>, db: &Database, tick: u64) {
    tracing::debug!("Processing interactions tick={tick}");

    let players: Vec<Player> = game_state
        .active_players
        .read()
        .await
        .values()
        .cloned()
        .collect();

    for player in players {
        process_player(game_state, db, &player).await;
    }
}

async fn process_player(game_state: &Arc<GameState>, db: &Database, player: &Player) {
    let interactions = game_state.mailboxes.drain(player.entity_id).await;
    for interaction in interactions {
        dispatch_interaction(game_state, db, player, interaction).await;
    }
}

async fn dispatch_interaction(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    interaction: Interaction,
) {
    match interaction {
        Interaction::Look => {
            look::process(game_state, db, player).await;
        }
        Interaction::Help => {
            help::process(game_state, player).await;
        }
        Interaction::Movement(Movement::TryDirection(direction)) => {
            movement::process(game_state, db, player, direction).await;
        }
        Interaction::Movement(Movement::Warp(_)) => {}
        Interaction::EngagementAction(action) => {
            let accepted = game_state
                .engagements
                .submit_action_for_entity(player.entity_id, action)
                .await;
            tracing::debug!(
                entity_id = player.entity_id,
                accepted,
                "engagement action submitted"
            );
        }
        Interaction::StartConversation => {
            conversation::process(game_state, player).await;
        }
    }
}
