use std::sync::Arc;

use crate::game::GameState;
use crate::game::engagement::battle;
use crate::game::player::Player;

pub async fn player_disconnected(game_state: &Arc<GameState>, player: &Player) {
    if let Some((engagement_id, surviving)) =
        battle::participants::remove_entity(&game_state.engagements.battles, player.entity_id).await
        && surviving <= 1
    {
        battle::end_battle(game_state, engagement_id, &[player.entity_id]).await;
    }

    game_state
        .active_characters
        .write()
        .await
        .remove(&player.entity_id);
    game_state
        .active_players
        .write()
        .await
        .remove(&player.client_id);
    game_state.mailboxes.remove(player.entity_id).await;
}
