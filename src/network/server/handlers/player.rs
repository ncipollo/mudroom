use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use tracing::info;

use crate::game::{Entity, EntityType, Location, next_id};
use crate::network::event::{NetworkEvent, PlayerInfo, PlayerListResponse};
use crate::network::server::state::{AppState, PlayerCreateBody, PlayerListBody, PlayerSelectBody};
use crate::persistence::{entity_repo, player_repo};

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
    let spawn = &state.game_state.mud_config.spawn;
    let location = Location {
        world_id: spawn.world_id.clone(),
        dungeon_id: spawn.dungeon_id.clone(),
        room_id: spawn.room_id.clone(),
    };
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

    let conns = state.connections.read().await;
    if let Some(client) = conns.get(&body.client_id) {
        let _ = client
            .personal_tx
            .send(NetworkEvent::PlayerSelected {
                client_id: body.client_id,
                player_id: player.id,
                player_name: player.name.clone(),
            })
            .await;
    }

    Ok(Json(PlayerInfo {
        id: player.id,
        name: player.name,
    }))
}
