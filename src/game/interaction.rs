pub mod conversation;
pub mod help;
pub mod look;
pub mod movement;
pub mod room_threats;

use std::sync::Arc;

use tracing;

use crate::game::component::interaction::Movement;
use crate::game::engagement::TurnAction;
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
        Interaction::Look => look::process(game_state, db, player).await,
        Interaction::Help => help::process(game_state, player).await,
        Interaction::Movement(m) => dispatch_movement(game_state, db, player, m).await,
        Interaction::EngagementAction(action) => {
            dispatch_engagement_action(game_state, player, action).await;
        }
        conv @ (Interaction::StartConversation { .. } | Interaction::EndConversation) => {
            dispatch_conversation(game_state, player, conv).await;
        }
        Interaction::JoinBattle { engagement_id } | Interaction::LeaveBattle { engagement_id } => {
            tracing::debug!(engagement_id, "battle interaction");
        }
    }
}

async fn dispatch_movement(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    movement: Movement,
) {
    match movement {
        Movement::TryDirection(direction) => {
            movement::process(game_state, db, player, direction).await;
        }
        Movement::Warp(_) => {}
    }
}

async fn dispatch_engagement_action(
    game_state: &Arc<GameState>,
    player: &Player,
    action: TurnAction,
) {
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

async fn dispatch_conversation(
    game_state: &Arc<GameState>,
    player: &Player,
    interaction: Interaction,
) {
    match interaction {
        Interaction::StartConversation { initial_message } => {
            conversation::process(game_state, player, initial_message).await;
        }
        Interaction::EndConversation => {
            conversation::end_player_conversation(game_state, player).await;
        }
        _ => {}
    }
}
