use std::sync::Arc;

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
        let descriptions: Vec<(EntityType, Option<String>)> = entities
            .values()
            .filter(|e| e.id != player.entity_id && e.location == location)
            .map(|e| (e.entity_type.clone(), e.description.clone()))
            .collect();
        (location, descriptions)
    };

    if let Ok(Some(room)) =
        room_repo::find_by_id(db.pool(), &location.dungeon_id, &location.room_id).await
    {
        messaging::message_room_description(&game_state.message_tx, player.id, &room);
    }

    for (entity_type, description) in entity_descriptions {
        let content = description
            .unwrap_or_else(|| format!("A {} is here.", entity_type_label(&entity_type)));
        messaging::message(&game_state.message_tx, player.id, content);
    }
}

fn entity_type_label(entity_type: &EntityType) -> &'static str {
    match entity_type {
        EntityType::Character => "character",
        EntityType::Object => "object",
        EntityType::Player => "player",
    }
}
