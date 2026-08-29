mod items;

use std::sync::Arc;

use tracing;

use crate::game::component::ItemDefinition;
use crate::game::component::description::Description;
use crate::game::config::item_config::select_by_name;
use crate::game::config::theme_config;
use crate::game::entity::character::CharacterType;
use crate::game::player::Player;
use crate::game::{GameState, messaging};
use crate::persistence::Database;
use crate::persistence::{room_repo, world_loot_repo};

pub async fn process(game_state: &Arc<GameState>, db: &Database, player: &Player, is_entry: bool) {
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

    items::send_item_descriptions(game_state, db, player, &location, is_entry).await;
}

fn character_type_label(character_type: &CharacterType) -> &'static str {
    match character_type {
        CharacterType::Character => "character",
        CharacterType::Enemy => "enemy",
        CharacterType::Player => "player",
    }
}

pub async fn process_at(game_state: &Arc<GameState>, db: &Database, player: &Player, target: &str) {
    let location = {
        let characters = game_state.active_characters.read().await;
        match characters.get(&player.entity_id) {
            Some(c) => c.location.clone(),
            None => return,
        }
    };

    let loot = match world_loot_repo::find_by_location(db.pool(), &location).await {
        Ok(loot) => loot,
        Err(e) => {
            tracing::error!("Failed to load world loot for look-at: {e}");
            return;
        }
    };

    let definitions = game_state.item_definitions.read().await;
    let matches: Vec<&ItemDefinition> = select_by_name(loot.iter(), target, |l| {
        definitions.get(&l.item_definition_id)
    })
    .into_iter()
    .filter_map(|l| definitions.get(&l.item_definition_id))
    .collect();

    respond_to_look_at(game_state, player, target, matches.as_slice());
}

fn respond_to_look_at(
    game_state: &Arc<GameState>,
    player: &Player,
    target: &str,
    matches: &[&ItemDefinition],
) {
    match matches {
        [] => messaging::message(
            &game_state.message_tx,
            player.id,
            format!("You don't see a '{target}' here."),
        ),
        [def] => {
            let theme = theme_config::resolve_theme_id(
                &game_state.themes,
                def.description.theme.as_deref(),
            );
            let content = def
                .description
                .text
                .clone()
                .unwrap_or_else(|| format!("You see nothing special about the {}.", def.name));
            messaging::message_themed(&game_state.message_tx, player.id, content, theme);
        }
        _ => messaging::message(
            &game_state.message_tx,
            player.id,
            format!("Which '{target}' do you mean?"),
        ),
    }
}
