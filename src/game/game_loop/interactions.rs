mod look;
mod movement;

use std::sync::Arc;

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
        let interactions = game_state.mailboxes.drain(player.entity_id).await;
        for interaction in interactions {
            match interaction {
                Interaction::Look => {
                    look::process(game_state, db, &player).await;
                }
                Interaction::Movement(Movement::TryDirection(direction)) => {
                    movement::process(game_state, db, &player, direction).await;
                }
                Interaction::Movement(Movement::Warp(_)) => {}
            }
        }
    }
}
