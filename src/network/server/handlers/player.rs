use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use tracing::info;

use crate::game::component::Attribute;
use crate::game::interaction::room_threats;
use crate::game::{Entity, EntityType, Location, Player};
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
    let mut entity = Entity::new(0, EntityType::Player, location);
    entity.name = body.name.clone();
    entity.attributes = default_player_attributes();
    let entity_id = entity_repo::insert(pool, &entity)
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

    activate_player(&state, &body.client_id, &player).await?;

    Ok(Json(PlayerInfo {
        id: player.id,
        name: player.name,
    }))
}

async fn activate_player(
    state: &AppState,
    client_id: &str,
    player: &Player,
) -> Result<(), StatusCode> {
    let pool = state.db.pool();
    let mut entity = entity_repo::find_by_id(pool, player.entity_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    entity.name = player.name.clone();

    let room_id = entity.location.room_id.clone();
    register_player_in_game_state(state, client_id, player, entity).await;
    notify_player_selected(state, client_id, player).await;
    room_threats::check_room_hostility(&state.game_state, player, &room_id).await;
    Ok(())
}

async fn register_player_in_game_state(
    state: &AppState,
    client_id: &str,
    player: &Player,
    entity: crate::game::Entity,
) {
    let pool = state.db.pool();
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
        .insert(client_id.to_string(), player.clone());
    if let Err(e) = state.game_state.sync_active_entities(pool).await {
        tracing::error!(error = %e, "Failed to sync active entities on player select");
    }
}

fn default_player_attributes() -> std::collections::HashMap<String, Attribute> {
    let mut attrs = std::collections::HashMap::new();
    attrs.insert(
        "hp".to_string(),
        Attribute::new("hp".to_string(), 0, 100, 100),
    );
    attrs.insert(
        "mp".to_string(),
        Attribute::new("mp".to_string(), 0, 50, 50),
    );
    attrs
}

async fn notify_player_selected(state: &AppState, client_id: &str, player: &Player) {
    let conns = state.connections.read().await;
    if let Some(client) = conns.get(client_id) {
        let _ = client
            .personal_tx
            .send(NetworkEvent::PlayerSelected {
                client_id: client_id.to_string(),
                player_id: player.id,
                player_name: player.name.clone(),
            })
            .await;
    }
}
