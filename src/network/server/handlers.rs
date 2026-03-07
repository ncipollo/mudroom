use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;

use super::state::{
    AppState, ConnectedClient, GuardedStream, PingBody, PlayerCreateBody, PlayerListBody,
    PlayerSelectBody, SessionEndBody, SessionStartBody, SseCleanupGuard, SseQuery,
};
use crate::game::{Description, Dungeon, Entity, EntityType, Location, Room, World, next_id};
use crate::network::event::{
    NetworkEvent, PlayerInfo, PlayerListResponse, ServerInfoResponse, SessionStartResponse,
};
use crate::persistence::{dungeon_repo, entity_repo, player_repo, room_repo, world_repo};

pub async fn server_info_handler(State(state): State<Arc<AppState>>) -> Json<ServerInfoResponse> {
    info!("GET /server/info");
    Json(ServerInfoResponse {
        server_id: state.server_session.id.clone(),
    })
}

pub async fn sse_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SseQuery>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    info!(client_id = %query.client_id, "GET /events - client subscribed to SSE");
    let rx = state.tx.subscribe();
    let inner = BroadcastStream::new(rx).filter_map(|result| {
        result.ok().and_then(|event| {
            serde_json::to_string(&event)
                .ok()
                .map(|data| Ok(Event::default().data(data)))
        })
    });
    let guard = SseCleanupGuard {
        client_id: query.client_id,
        connections: state.connections.clone(),
        tx: state.tx.clone(),
    };
    let stream = GuardedStream {
        inner,
        _guard: guard,
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub async fn session_start_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SessionStartBody>,
) -> Json<SessionStartResponse> {
    let client_id = body
        .client_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    info!(client_id = %client_id, "POST /session/start");
    state.connections.write().await.insert(
        client_id.clone(),
        ConnectedClient {
            last_ping: Instant::now(),
        },
    );
    let _ = state.tx.send(NetworkEvent::StartSession {
        session_id: client_id.clone(),
    });
    Json(SessionStartResponse {
        client_id,
        server_id: state.server_session.id.clone(),
    })
}

pub async fn ping_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PingBody>,
) -> &'static str {
    info!(client_id = %body.client_id, "POST /ping");
    if let Some(client) = state.connections.write().await.get_mut(&body.client_id) {
        client.last_ping = Instant::now();
    }
    let _ = state.tx.send(NetworkEvent::Pong);
    "ok"
}

pub async fn session_end_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SessionEndBody>,
) -> &'static str {
    info!(session_id = %body.session_id, "POST /session/end");
    state.connections.write().await.remove(&body.session_id);
    let _ = state.tx.send(NetworkEvent::EndSession {
        session_id: body.session_id,
    });
    "ok"
}

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
