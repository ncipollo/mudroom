use std::sync::Arc;

use crate::game::character::CharacterType;
use crate::game::component::description::Description;
use crate::game::config::theme_config;
use crate::game::player::Player;
use crate::game::{GameState, messaging};
use crate::persistence::Database;
use crate::persistence::room_repo;

pub async fn process(game_state: &Arc<GameState>, db: &Database, player: &Player) {
    let (location, character_descriptions) = {
        let characters = game_state.active_characters.read().await;
        let location = match characters.get(&player.entity_id) {
            Some(c) => c.location.clone(),
            None => return,
        };
        let descriptions: Vec<(CharacterType, Description)> = characters
            .values()
            .filter(|c| c.id != player.entity_id && c.location == location)
            .map(|c| (c.character_type.clone(), c.description.clone()))
            .collect();
        (location, descriptions)
    };

    if let Ok(Some(room)) =
        room_repo::find_by_id(db.pool(), &location.dungeon_id, &location.room_id).await
    {
        let theme =
            theme_config::resolve_theme_id(&game_state.themes, room.description.theme.as_deref());
        messaging::message_room_description(&game_state.message_tx, player.id, &room, theme);
    }

    for (character_type, description) in character_descriptions {
        let theme =
            theme_config::resolve_theme_id(&game_state.themes, description.theme.as_deref());
        let content = description
            .text
            .unwrap_or_else(|| format!("A {} is here.", character_type_label(&character_type)));
        messaging::message_themed(&game_state.message_tx, player.id, content, theme);
    }
}

fn character_type_label(character_type: &CharacterType) -> &'static str {
    match character_type {
        CharacterType::Character => "character",
        CharacterType::Enemy => "enemy",
        CharacterType::Player => "player",
    }
}
