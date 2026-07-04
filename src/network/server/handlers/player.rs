use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use tracing::info;

use crate::game::component::{Ability, Attribute};
use crate::game::config::class_config::ClassConfig;
use crate::game::{Entity, EntityType, Location, Player};
use crate::network::event::{
    ClassInfo, ClassListResponse, NetworkEvent, PlayerInfo, PlayerListResponse,
};
use crate::network::server::state::{AppState, PlayerCreateBody, PlayerListBody, PlayerSelectBody};
use crate::persistence::{ability_repo, entity_repo, player_repo};

pub async fn player_classes_handler(State(state): State<Arc<AppState>>) -> Json<ClassListResponse> {
    info!("GET /players/classes");
    let classes = state
        .game_state
        .classes
        .iter()
        .map(|(id, c)| ClassInfo {
            id: id.clone(),
            name: c.name.clone(),
            description: c.description.clone(),
        })
        .collect();
    Json(ClassListResponse { classes })
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
    let spawn = &state.game_state.mud_config.spawn;
    let location = Location {
        world_id: spawn.world_id.clone(),
        dungeon_id: spawn.dungeon_id.clone(),
        room_id: spawn.room_id.clone(),
    };
    let mut entity = Entity::new(0, EntityType::Player, location);
    entity.name = body.name.clone();
    let (attributes, innate_abilities) = resolve_class_data(&state, &body)?;
    entity.attributes = attributes;
    let entity_id = entity_repo::insert(pool, &entity)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    apply_class_abilities(pool, entity_id, &innate_abilities).await?;
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

    register_player_in_game_state(state, client_id, player, entity).await;
    notify_player_selected(state, client_id, player).await;
    Ok(())
}

async fn register_player_in_game_state(
    state: &AppState,
    client_id: &str,
    player: &Player,
    entity: Entity,
) {
    state
        .game_state
        .push_pending_activation(entity, player.clone(), client_id.to_string())
        .await;
}

fn resolve_class_data(
    state: &AppState,
    body: &PlayerCreateBody,
) -> Result<(HashMap<String, Attribute>, Vec<Ability>), StatusCode> {
    if let Some(class_id) = &body.class_id {
        let class = state
            .game_state
            .classes
            .get(class_id)
            .ok_or(StatusCode::BAD_REQUEST)?;
        Ok((class_attributes(class), class.innate_abilities.clone()))
    } else {
        Ok((default_player_attributes(), vec![]))
    }
}

fn class_attributes(class: &ClassConfig) -> HashMap<String, Attribute> {
    class
        .attributes
        .iter()
        .map(|sa| {
            (
                sa.definition_id.clone(),
                Attribute::new(
                    sa.definition_id.clone(),
                    sa.min_value,
                    sa.max_value,
                    sa.current_value,
                ),
            )
        })
        .collect()
}

async fn apply_class_abilities(
    pool: &sqlx::SqlitePool,
    entity_id: i64,
    abilities: &[Ability],
) -> Result<(), StatusCode> {
    if abilities.is_empty() {
        return Ok(());
    }
    for ability in abilities {
        ability_repo::upsert(pool, ability)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let ids: Vec<&str> = abilities.iter().map(|a| a.id.as_str()).collect();
    ability_repo::set_entity_abilities(pool, entity_id, &ids)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}

fn default_player_attributes() -> HashMap<String, Attribute> {
    let mut attrs = HashMap::new();
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
                entity_id: player.entity_id,
            })
            .await;
    }
}
