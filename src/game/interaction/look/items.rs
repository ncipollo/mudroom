use std::sync::Arc;

use tracing;

use crate::game::component::Location;
use crate::game::config::{RespawnMode, theme_config};
use crate::game::player::Player;
use crate::game::{GameState, messaging};
use crate::persistence::Database;
use crate::persistence::{PersistenceError, world_loot_repo};

pub async fn send_item_descriptions(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    location: &Location,
    is_entry: bool,
) {
    if is_entry {
        respawn_on_visit(db, &game_state.mud_config.world_loot.respawn_mode, location).await;
    }

    let loot = match world_loot_repo::find_by_location(db.pool(), location).await {
        Ok(loot) => loot,
        Err(e) => {
            tracing::error!("Failed to load world loot for look: {e}");
            return;
        }
    };

    let definitions = game_state.item_definitions.read().await;
    for def in loot
        .iter()
        .filter_map(|l| definitions.get(&l.item_definition_id))
    {
        let theme =
            theme_config::resolve_theme_id(&game_state.themes, def.description.theme.as_deref());
        let content = def
            .description
            .text
            .clone()
            .unwrap_or_else(|| format!("A {} lies here.", def.name));
        messaging::message_themed(&game_state.message_tx, player.id, content, theme);
    }
}

async fn respawn_on_visit(db: &Database, mode: &RespawnMode, location: &Location) {
    let result = match mode {
        RespawnMode::OnRoomVisit => world_loot_repo::respawn_room(db.pool(), location).await,
        RespawnMode::OnDungeonVisit => {
            world_loot_repo::respawn_dungeon(db.pool(), &location.world_id, &location.dungeon_id)
                .await
        }
        RespawnMode::OnGameReboot | RespawnMode::Never => Ok(()),
    };
    log_on_error(result, "respawn world loot on visit");
}

fn log_on_error<T>(result: Result<T, PersistenceError>, action: &str) {
    if let Err(e) = result {
        tracing::error!("Failed to {action}: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::{Description, EquippedBonuses, ItemDefinition, ItemUseType};
    use crate::game::entity::character::{Character, CharacterType};
    use crate::game::messaging::Message;
    use crate::game::{Dungeon, Room, World};
    use crate::persistence::{dungeon_repo, item_repo, room_repo, world_repo};

    fn test_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    fn test_player() -> Player {
        Player {
            id: 1,
            client_id: "client".to_string(),
            name: "Hero".to_string(),
            entity_id: 1,
        }
    }

    async fn setup_db(db: &Database) {
        world_repo::insert(db.pool(), &World::new("w1".to_string()))
            .await
            .unwrap();
        dungeon_repo::insert(db.pool(), &Dungeon::new("d1".to_string()), "w1")
            .await
            .unwrap();
        let room = Room::new("r1".to_string(), Description::new(None));
        room_repo::insert(db.pool(), &room, "d1").await.unwrap();

        let def = ItemDefinition {
            id: "medicine".to_string(),
            name: "Medicine".to_string(),
            description: Description::new(Some("A restorative brew.".to_string())),
            use_type: ItemUseType::Used,
            item_type: "consumable".to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects: vec![],
            alternate_names: vec![],
        };
        item_repo::upsert_definition(db.pool(), &def).await.unwrap();
    }

    async fn setup_game_state(respawn_mode: RespawnMode) -> Arc<GameState> {
        let mut game_state = GameState::load(None).unwrap();
        game_state.mud_config.world_loot.respawn_mode = respawn_mode;
        let game_state = Arc::new(game_state);
        game_state.item_definitions.write().await.insert(
            "medicine".to_string(),
            ItemDefinition {
                id: "medicine".to_string(),
                name: "Medicine".to_string(),
                description: Description::new(Some("A restorative brew.".to_string())),
                use_type: ItemUseType::Used,
                item_type: "consumable".to_string(),
                equipped_bonuses: EquippedBonuses::default(),
                use_effects: vec![],
                alternate_names: vec![],
            },
        );
        game_state
            .active_characters
            .write()
            .await
            .insert(1, Character::new(1, CharacterType::Player, test_location()));
        game_state
    }

    #[tokio::test]
    async fn send_item_descriptions_narrates_loot_in_room() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_db(&db).await;
        let game_state = setup_game_state(RespawnMode::Never).await;
        world_loot_repo::insert_config_loot_if_missing(db.pool(), &test_location(), "medicine")
            .await
            .unwrap();

        let mut rx = game_state.message_tx.subscribe();
        send_item_descriptions(&game_state, &db, &test_player(), &test_location(), true).await;

        let msg = rx.try_recv().expect("expected an item description message");
        match msg.message {
            Message::Complete { content, .. } => {
                assert_eq!(content, "A restorative brew.");
            }
            other => panic!("expected Complete message, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn send_item_descriptions_skips_taken_loot() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_db(&db).await;
        let game_state = setup_game_state(RespawnMode::Never).await;
        let (id, _) =
            world_loot_repo::insert_config_loot_if_missing(db.pool(), &test_location(), "medicine")
                .await
                .unwrap();
        world_loot_repo::mark_taken(db.pool(), id).await.unwrap();

        let mut rx = game_state.message_tx.subscribe();
        send_item_descriptions(&game_state, &db, &test_player(), &test_location(), true).await;

        assert!(rx.try_recv().is_err(), "expected no item description");
    }

    #[tokio::test]
    async fn on_room_visit_respawns_taken_loot_on_entry() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_db(&db).await;
        let game_state = setup_game_state(RespawnMode::OnRoomVisit).await;
        let (id, _) =
            world_loot_repo::insert_config_loot_if_missing(db.pool(), &test_location(), "medicine")
                .await
                .unwrap();
        world_loot_repo::mark_taken(db.pool(), id).await.unwrap();

        send_item_descriptions(&game_state, &db, &test_player(), &test_location(), true).await;

        let loot = world_loot_repo::find_by_location(db.pool(), &test_location())
            .await
            .unwrap();
        assert_eq!(loot.len(), 1);
    }

    #[tokio::test]
    async fn on_room_visit_does_not_respawn_when_not_entry() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_db(&db).await;
        let game_state = setup_game_state(RespawnMode::OnRoomVisit).await;
        let (id, _) =
            world_loot_repo::insert_config_loot_if_missing(db.pool(), &test_location(), "medicine")
                .await
                .unwrap();
        world_loot_repo::mark_taken(db.pool(), id).await.unwrap();

        send_item_descriptions(&game_state, &db, &test_player(), &test_location(), false).await;

        let loot = world_loot_repo::find_by_location(db.pool(), &test_location())
            .await
            .unwrap();
        assert!(loot.is_empty());
    }

    #[tokio::test]
    async fn never_mode_does_not_respawn_taken_loot() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_db(&db).await;
        let game_state = setup_game_state(RespawnMode::Never).await;
        let (id, _) =
            world_loot_repo::insert_config_loot_if_missing(db.pool(), &test_location(), "medicine")
                .await
                .unwrap();
        world_loot_repo::mark_taken(db.pool(), id).await.unwrap();

        send_item_descriptions(&game_state, &db, &test_player(), &test_location(), true).await;

        let loot = world_loot_repo::find_by_location(db.pool(), &test_location())
            .await
            .unwrap();
        assert!(loot.is_empty());
    }
}
