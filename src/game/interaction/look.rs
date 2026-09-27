mod items;
mod matching;

use std::sync::Arc;

use tracing;

use crate::game::component::description::Description;
use crate::game::config::theme_config;
use crate::game::entity::character::CharacterType;
use crate::game::player::Player;
use crate::game::{GameState, messaging};
use crate::persistence::Database;
use crate::persistence::room_repo;
use matching::{LookMatch, matching_targets};

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

    let matches = match matching_targets(game_state, db, &location, target).await {
        Ok(matches) => matches,
        Err(e) => {
            tracing::error!("Failed to load look-at targets: {e}");
            return;
        }
    };

    respond_to_look_at(game_state, player, target, matches.as_slice()).await;
}

async fn respond_to_look_at(
    game_state: &Arc<GameState>,
    player: &Player,
    target: &str,
    matches: &[LookMatch],
) {
    match matches {
        [] => messaging::message(
            &game_state.message_tx,
            player.id,
            format!("You don't see a '{target}' here."),
        ),
        [look_match] => send_look_match_description(game_state, player, look_match).await,
        _ => messaging::message(
            &game_state.message_tx,
            player.id,
            format!("Which '{target}' do you mean?"),
        ),
    }
}

/// Sends the themed message(s) for a single unambiguous look-at match: the state
/// description, combined with its held items into one sentence when the matched state
/// configures an `item_summary` template, or followed by a separate themed message per
/// held item (today's behavior) when it doesn't.
async fn send_look_match_description(
    game_state: &Arc<GameState>,
    player: &Player,
    look_match: &LookMatch,
) {
    let theme =
        theme_config::resolve_theme_id(&game_state.themes, look_match.description.theme.as_deref());
    let desc_text = look_match
        .description
        .text
        .clone()
        .unwrap_or_else(|| format!("You see nothing special about the {}.", look_match.name));

    let content = match &look_match.item_summary {
        Some(template) if !look_match.items.is_empty() => {
            match items::format_item_summary(game_state, template, &look_match.items).await {
                Some(summary) => format!("{desc_text}{summary}"),
                None => desc_text.clone(),
            }
        }
        _ => desc_text.clone(),
    };
    messaging::message_themed(&game_state.message_tx, player.id, content, theme);

    if look_match.item_summary.is_none() && !look_match.items.is_empty() {
        items::send_feature_item_descriptions(
            game_state,
            player,
            &look_match.name,
            &look_match.items,
        )
        .await;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::game::component::{Description, EquippedBonuses, ItemDefinition, ItemUseType};
    use crate::game::entity::character::Character;
    use crate::game::map::universe::room_feature::FeatureState;
    use crate::game::map::universe::room_feature::RoomFeature;
    use crate::game::messaging::Message;
    use crate::game::{Dungeon, Room, World};
    use crate::persistence::{dungeon_repo, item_repo, room_feature_repo, room_repo, world_repo};

    fn test_location() -> crate::game::component::Location {
        crate::game::component::Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    fn test_player(entity_id: i64) -> Player {
        Player {
            id: 1,
            client_id: "client".to_string(),
            name: "Hero".to_string(),
            entity_id,
        }
    }

    async fn setup_world(db: &Database) {
        world_repo::insert(db.pool(), &World::new("w1".to_string()))
            .await
            .unwrap();
        dungeon_repo::insert(db.pool(), &Dungeon::new("d1".to_string()), "w1")
            .await
            .unwrap();
        room_repo::insert(
            db.pool(),
            &Room::new("r1".to_string(), Description::new(None)),
            "d1",
        )
        .await
        .unwrap();
    }

    async fn game_state_with_character(entity_id: i64) -> Arc<GameState> {
        let game_state = Arc::new(GameState::load(None).unwrap());
        game_state.active_characters.write().await.insert(
            entity_id,
            Character::new(entity_id, CharacterType::Player, test_location()),
        );
        game_state
    }

    fn chest_feature(items: Vec<String>, item_summary: Option<String>) -> RoomFeature {
        let mut states = HashMap::new();
        states.insert(
            "open".to_string(),
            FeatureState {
                description: Description::new(Some("An open oak chest.".to_string())),
                items,
                interact_script: None,
                interact_next_state: None,
                alt_verbs: vec![],
                item_summary,
            },
        );
        RoomFeature {
            id: "chest".to_string(),
            name: "Oak Chest".to_string(),
            default_state: "open".to_string(),
            states,
            alternate_names: vec![],
        }
    }

    async fn seed_feature(db: &Database, feature: &RoomFeature) {
        room_feature_repo::upsert_definition(db.pool(), feature)
            .await
            .unwrap();
        let items = feature.states["open"].items.clone();
        room_feature_repo::insert_placement_if_missing(
            db.pool(),
            &test_location(),
            &feature.id,
            "open",
            &items,
        )
        .await
        .unwrap();
    }

    async fn seed_item(game_state: &Arc<GameState>, db: &Database, id: &str, name: &str) {
        let def = ItemDefinition {
            id: id.to_string(),
            name: name.to_string(),
            description: Description::new(None),
            use_type: ItemUseType::Passive,
            item_type: "consumable".to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects: vec![],
            alternate_names: vec![],
        };
        item_repo::upsert_definition(db.pool(), &def).await.unwrap();
        game_state
            .item_definitions
            .write()
            .await
            .insert(def.id.clone(), def);
    }

    #[tokio::test]
    async fn look_at_feature_with_items_and_no_summary_describes_state_then_each_item() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let game_state = game_state_with_character(1).await;
        let feature = chest_feature(vec!["medicine".to_string()], None);
        seed_feature(&db, &feature).await;
        seed_item(&game_state, &db, "medicine", "Medicine").await;

        let mut rx = game_state.message_tx.subscribe();
        process_at(&game_state, &db, &test_player(1), "oak chest").await;

        let state_msg = rx.try_recv().expect("expected state description");
        match state_msg.message {
            Message::Complete { content, .. } => {
                assert_eq!(content, "An open oak chest.");
            }
            other => panic!("expected Complete message, got {other:?}"),
        }

        let item_msg = rx.try_recv().expect("expected item description");
        match item_msg.message {
            Message::Complete { content, .. } => {
                assert_eq!(content, "A Medicine is inside the Oak Chest.");
            }
            other => panic!("expected Complete message, got {other:?}"),
        }

        assert!(rx.try_recv().is_err(), "expected no further messages");
    }

    #[tokio::test]
    async fn look_at_feature_with_no_items_sends_only_state_description() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let game_state = game_state_with_character(1).await;
        let feature = chest_feature(vec![], None);
        seed_feature(&db, &feature).await;

        let mut rx = game_state.message_tx.subscribe();
        process_at(&game_state, &db, &test_player(1), "oak chest").await;

        rx.try_recv().expect("expected state description");
        assert!(rx.try_recv().is_err(), "expected no item description");
    }

    #[tokio::test]
    async fn look_at_feature_with_item_summary_sends_one_combined_message() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let game_state = game_state_with_character(1).await;
        let feature = chest_feature(
            vec!["medicine".to_string()],
            Some(", inside there is {items}.".to_string()),
        );
        seed_feature(&db, &feature).await;
        seed_item(&game_state, &db, "medicine", "Medicine").await;

        let mut rx = game_state.message_tx.subscribe();
        process_at(&game_state, &db, &test_player(1), "oak chest").await;

        let msg = rx.try_recv().expect("expected a combined message");
        match msg.message {
            Message::Complete { content, .. } => {
                assert_eq!(content, "An open oak chest., inside there is a Medicine.");
            }
            other => panic!("expected Complete message, got {other:?}"),
        }

        assert!(rx.try_recv().is_err(), "expected no further messages");
    }

    #[tokio::test]
    async fn look_at_feature_with_item_summary_but_unresolved_item_sends_only_state_description() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let game_state = game_state_with_character(1).await;
        let feature = chest_feature(
            vec!["unknown".to_string()],
            Some(", inside there is {items}.".to_string()),
        );
        seed_feature(&db, &feature).await;

        let mut rx = game_state.message_tx.subscribe();
        process_at(&game_state, &db, &test_player(1), "oak chest").await;

        let msg = rx.try_recv().expect("expected state description");
        match msg.message {
            Message::Complete { content, .. } => {
                assert_eq!(content, "An open oak chest.");
            }
            other => panic!("expected Complete message, got {other:?}"),
        }

        assert!(rx.try_recv().is_err(), "expected no further messages");
    }
}
