use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use tracing::info;

use crate::game::{Description, Dungeon, Entity, EntityType, Location, Room, World, next_id};
use crate::network::event::{NetworkEvent, PlayerInfo, PlayerListResponse};
use crate::network::server::state::{AppState, PlayerCreateBody, PlayerListBody, PlayerSelectBody};
use crate::persistence::{dungeon_repo, entity_repo, player_repo, room_repo, world_repo};

const DEFAULT_WORLD_ID: &str = "default";
const DEFAULT_DUNGEON_ID: &str = "default";
const DEFAULT_ROOM_ID: &str = "default";

async fn ensure_default_spawn(pool: &sqlx::SqlitePool) -> Result<Location, StatusCode> {
    let world = World::new(DEFAULT_WORLD_ID.to_string());
    world_repo::insert(pool, &world)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let dungeon = Dungeon::new(DEFAULT_DUNGEON_ID.to_string());
    dungeon_repo::insert(pool, &dungeon, DEFAULT_WORLD_ID)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let room = Room::new(DEFAULT_ROOM_ID.to_string(), Description::new(None));
    room_repo::insert(pool, &room, DEFAULT_DUNGEON_ID)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Location {
        world_id: DEFAULT_WORLD_ID.to_string(),
        dungeon_id: DEFAULT_DUNGEON_ID.to_string(),
        room_id: DEFAULT_ROOM_ID.to_string(),
    })
}

pub async fn player_list_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PlayerListBody>,
) -> Result<Json<PlayerListResponse>, StatusCode> {
    info!(client_id = %body.client_id, "POST /players/list");
    let players = player_repo::find_by_client_id(state.db.pool(), &body.client_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let player_infos = players
        .into_iter()
        .map(|p| PlayerInfo {
            id: p.id,
            name: p.name,
        })
        .collect();
    Ok(Json(PlayerListResponse {
        players: player_infos,
    }))
}

pub async fn player_create_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PlayerCreateBody>,
) -> Result<Json<PlayerInfo>, StatusCode> {
    info!(client_id = %body.client_id, name = %body.name, "POST /players/create");
    let pool = state.db.pool();
    let location = ensure_default_spawn(pool).await?;
    let entity_id = next_id();
    let entity = Entity::new(entity_id, EntityType::Player, location);
    entity_repo::insert(pool, &entity)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let player_id = player_repo::insert(pool, &body.client_id, &body.name, entity_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(PlayerInfo {
        id: player_id,
        name: body.name,
    }))
}

pub async fn player_select_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PlayerSelectBody>,
) -> Result<Json<PlayerInfo>, StatusCode> {
    info!(client_id = %body.client_id, player_id = %body.player_id, "POST /players/select");
    let pool = state.db.pool();
    let player = player_repo::find_by_id(pool, body.player_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if player.client_id != body.client_id {
        return Err(StatusCode::FORBIDDEN);
    }

    let entity = entity_repo::find_by_id(pool, player.entity_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    state
        .game_state
        .active_entities
        .write()
        .await
        .insert(entity.id, entity);

    state
        .game_state
        .active_players
        .write()
        .await
        .insert(body.client_id.clone(), player.clone());

    let _ = state.tx.send(NetworkEvent::PlayerSelected {
        client_id: body.client_id,
        player_id: player.id,
        player_name: player.name.clone(),
    });

    Ok(Json(PlayerInfo {
        id: player.id,
        name: player.name,
    }))
}
