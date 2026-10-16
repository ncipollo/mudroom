use std::sync::Arc;

use boa_engine::object::ObjectInitializer;
use boa_engine::{Context, JsResult, js_string};
use sqlx::SqlitePool;
use thiserror::Error;

use crate::game::GameState;
use crate::game::component::Location;
use crate::game::game_state::EntitySpawnError;
use crate::persistence::PersistenceError;

use super::callback::{self, Captures};

mod scope;
pub use scope::ScriptScope;

#[derive(Debug, Error)]
pub enum RoomApiError {
    #[error("location {0:?} is outside this script's scope")]
    OutOfScope(Location),
    #[error(transparent)]
    Spawn(#[from] EntitySpawnError),
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
}

impl RoomApiError {
    fn out_of_scope(target: &Location) -> Self {
        RoomApiError::OutOfScope(target.clone())
    }
}

async fn spawn_entity_handler(
    (config_id, location): (String, Location),
    captures: Captures<(Arc<GameState>, SqlitePool, ScriptScope)>,
) -> Result<i64, RoomApiError> {
    let Captures((game_state, pool, scope)) = captures;
    scope.check(&pool, &location).await?;
    Ok(game_state
        .spawn_character(&pool, &config_id, location)
        .await?)
}

/// Installs a `room` global with an awaitable `spawnEntity(configId, location)`,
/// scoped to `scope`. Build once per script execution context and call from each
/// trigger's own path (room-feature/dungeon/world onLoad/onInteract/onTick) with
/// that trigger's own scope, rather than copy-pasting this wiring.
pub fn install_room_api(
    context: &mut Context,
    game_state: Arc<GameState>,
    pool: SqlitePool,
    scope: ScriptScope,
) -> JsResult<()> {
    let native =
        callback::native_async_fn_multi(Captures((game_state, pool, scope)), spawn_entity_handler);
    let room = ObjectInitializer::new(context)
        .function(native, js_string!("spawnEntity"), 1)
        .build();
    context
        .global_object()
        .set(js_string!("room"), room, true, context)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::game::component::description::Description;
    use crate::game::config::character_config::{CharacterConfig, CharacterTypeConfig};
    use crate::game::script::execution;
    use crate::game::{Dungeon, Room, World};
    use crate::persistence::{Database, character_repo, dungeon_repo, room_repo, world_repo};

    use super::*;

    fn home_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    async fn seed(db: &Database) {
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

    fn zombie_config() -> CharacterConfig {
        CharacterConfig {
            id: Some("entities/zombie".to_string()),
            name: Some("Zombie".to_string()),
            entity_type: CharacterTypeConfig::Enemy,
            description: Description::new(Some("A shambling corpse.".to_string())),
            persona: None,
            attributes: Vec::new(),
            entity_effects: Vec::new(),
            innate_abilities: Vec::new(),
            factions: Vec::new(),
            faction_relations: None,
            battle_ai: Default::default(),
        }
    }

    fn game_state_with_zombie_config() -> GameState {
        let mut game_state = GameState::load(None).unwrap();
        game_state
            .character_configs
            .insert("entities/zombie".to_string(), zombie_config());
        game_state
    }

    fn spawn_script(location_json: &str) -> String {
        format!("globalThis.result = await room.spawnEntity(\"entities/zombie\", {location_json});")
    }

    fn location_json(location: &Location) -> String {
        format!(
            "{{ world_id: \"{}\", dungeon_id: \"{}\", room_id: \"{}\" }}",
            location.world_id, location.dungeon_id, location.room_id
        )
    }

    #[tokio::test]
    async fn spawn_entity_round_trips_into_persisted_and_active_character() {
        let db = Database::connect_in_memory().await.unwrap();
        seed(&db).await;
        let game_state = Arc::new(game_state_with_zombie_config());
        let (mut context, executor) = execution::new_async_context().unwrap();
        install_room_api(
            &mut context,
            game_state.clone(),
            db.pool().clone(),
            ScriptScope::Room(home_location()),
        )
        .unwrap();

        execution::run_async(
            &mut context,
            &executor,
            &spawn_script(&location_json(&home_location())),
        )
        .await
        .unwrap();

        let id = context
            .global_object()
            .get(js_string!("result"), &mut context)
            .unwrap()
            .to_i32(&mut context)
            .unwrap() as i64;

        let persisted = character_repo::find_by_id(db.pool(), id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(persisted.config_id.as_deref(), Some("entities/zombie"));

        let active = game_state.active_characters.read().await;
        assert!(active.contains_key(&id));
    }

    #[tokio::test]
    async fn two_sequential_awaited_spawns_both_resolve_in_one_run() {
        let db = Database::connect_in_memory().await.unwrap();
        seed(&db).await;
        let game_state = Arc::new(game_state_with_zombie_config());
        let (mut context, executor) = execution::new_async_context().unwrap();
        install_room_api(
            &mut context,
            game_state.clone(),
            db.pool().clone(),
            ScriptScope::Room(home_location()),
        )
        .unwrap();

        let location = location_json(&home_location());
        let script = format!(
            "const a = await room.spawnEntity(\"entities/zombie\", {location});
             const b = await room.spawnEntity(\"entities/zombie\", {location});
             globalThis.result = [a, b];"
        );

        execution::run_async(&mut context, &executor, &script)
            .await
            .unwrap();

        let active = game_state.active_characters.read().await;
        assert_eq!(active.len(), 2);
    }

    #[tokio::test]
    async fn unknown_config_id_rejects() {
        let db = Database::connect_in_memory().await.unwrap();
        seed(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        let (mut context, executor) = execution::new_async_context().unwrap();
        install_room_api(
            &mut context,
            game_state,
            db.pool().clone(),
            ScriptScope::Room(home_location()),
        )
        .unwrap();

        let script = format!(
            "await room.spawnEntity(\"entities/nope\", {});",
            location_json(&home_location())
        );
        let result = execution::run_async(&mut context, &executor, &script).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn malformed_args_reject_as_type_error() {
        let db = Database::connect_in_memory().await.unwrap();
        seed(&db).await;
        let game_state = Arc::new(game_state_with_zombie_config());
        let (mut context, executor) = execution::new_async_context().unwrap();
        install_room_api(
            &mut context,
            game_state,
            db.pool().clone(),
            ScriptScope::Room(home_location()),
        )
        .unwrap();

        let result =
            execution::run_async(&mut context, &executor, "await room.spawnEntity();").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn out_of_scope_target_rejects() {
        let db = Database::connect_in_memory().await.unwrap();
        seed(&db).await;
        let game_state = Arc::new(game_state_with_zombie_config());
        let (mut context, executor) = execution::new_async_context().unwrap();
        install_room_api(
            &mut context,
            game_state,
            db.pool().clone(),
            ScriptScope::Room(home_location()),
        )
        .unwrap();

        let other = Location {
            room_id: "somewhere-else".to_string(),
            ..home_location()
        };
        let script = format!(
            "await room.spawnEntity(\"entities/zombie\", {});",
            location_json(&other)
        );
        let result = execution::run_async(&mut context, &executor, &script).await;

        assert!(result.is_err());
    }
}
