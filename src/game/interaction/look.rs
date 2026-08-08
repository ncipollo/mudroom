use std::sync::Arc;

use crate::game::component::description::Description;
use crate::game::config::theme_config;
use crate::game::entity::EntityType;
use crate::game::player::Player;
use crate::game::{GameState, messaging};
use crate::persistence::Database;
use crate::persistence::room_repo;

pub async fn process(game_state: &Arc<GameState>, db: &Database, player: &Player) {
    let (location, entity_descriptions) = {
        let entities = game_state.active_entities.read().await;
        let location = match entities.get(&player.entity_id) {
            Some(e) => e.location.clone(),
            None => return,
        };
        let descriptions: Vec<(EntityType, Description)> = entities
            .values()
            .filter(|e| e.id != player.entity_id && e.location == location)
            .map(|e| (e.entity_type.clone(), e.description.clone()))
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

    for (entity_type, description) in entity_descriptions {
        let theme =
            theme_config::resolve_theme_id(&game_state.themes, description.theme.as_deref());
        let content = description
            .text
            .unwrap_or_else(|| format!("A {} is here.", entity_type_label(&entity_type)));
        messaging::message_themed(&game_state.message_tx, player.id, content, theme);
    }
}

fn entity_type_label(entity_type: &EntityType) -> &'static str {
    match entity_type {
        EntityType::Character => "character",
        EntityType::Enemy => "enemy",
        EntityType::Object => "object",
        EntityType::Player => "player",
    }
}
