use std::sync::Arc;

use crate::game::player::Player;
use crate::game::{GameState, messaging};
use crate::persistence::Database;
use crate::persistence::room_repo;

pub async fn process(game_state: &Arc<GameState>, db: &Database, player: &Player) {
    let location = {
        let entities = game_state.active_entities.read().await;
        match entities.get(&player.entity_id) {
            Some(e) => e.location.clone(),
            None => return,
        }
    };

    if let Ok(Some(room)) =
        room_repo::find_by_id(db.pool(), &location.dungeon_id, &location.room_id).await
    {
        messaging::message_room_description(&game_state.message_tx, player.id, &room);
    }
}
