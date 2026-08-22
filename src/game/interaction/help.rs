use std::sync::Arc;

use crate::game::player::Player;
use crate::game::{GameState, messaging};

pub async fn process(game_state: &Arc<GameState>, player: &Player) {
    let help_text = r"Commands:
  n/north, s/south, e/east, w/west - Move
  l/look - Examine current room
  look at <item> / look <item> / l <item> - Examine an item
  take <item> - Pick up an item
  speak/talk/say [message] - Talk to someone nearby
  attack - Join a battle
  help - Show this help";
    messaging::message(&game_state.message_tx, player.id, help_text);
}
