use std::sync::Arc;

use crate::game::player::Player;
use crate::game::{GameState, messaging};

pub async fn process(game_state: &Arc<GameState>, player: &Player) {
    let help_text = r"Commands:
  n/north, s/south, e/east, w/west - Move
  l/look - Examine current room
  h/help - Show this help";
    messaging::message(&game_state.message_tx, player.id, help_text);
}
