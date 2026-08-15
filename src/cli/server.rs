use std::path::{Path, PathBuf};

use crate::{game, network, paths, persistence};

pub async fn run(
    name: Option<String>,
    config: Option<String>,
    reload_maps: bool,
    debug: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::logging::init_tracing(debug);
    let (server_session, config_path_buf) = init_server_session(name, config).await?;
    let (game_state, db) = init_game_resources(&server_session, config_path_buf.as_deref()).await?;
    if reload_maps || game::should_auto_load(db.pool()).await? {
        load_maps_into_db(&db, config_path_buf.as_deref(), reload_maps, &game_state).await?;
    }
    game_state.refresh_definition_caches(db.pool()).await?;
    serve_and_wait(server_session, game_state, db).await
}

async fn init_server_session(
    name: Option<String>,
    config: Option<String>,
) -> Result<(network::session::ServerSession, Option<PathBuf>), Box<dyn std::error::Error>> {
    let server_session = create_and_load_session(name).await?;
    let config_path_buf = find_config_path(config);
    Ok((server_session, config_path_buf))
}

async fn create_and_load_session(
    name: Option<String>,
) -> Result<network::session::ServerSession, Box<dyn std::error::Error>> {
    paths::create_session_base_dirs().await?;
    let session = network::session::ServerSession::load_or_create(name).await?;
    tracing::info!("Server session loaded: {} {:?}", session.id, session.name);
    Ok(session)
}

fn find_config_path(config: Option<String>) -> Option<PathBuf> {
    let path = config
        .as_deref()
        .map(PathBuf::from)
        .or_else(paths::find_config_dir);
    tracing::info!("Config directory resolved: {:?}", path);
    path
}

async fn init_game_resources(
    server_session: &network::session::ServerSession,
    config_path: Option<&Path>,
) -> Result<(game::GameState, persistence::Database), Box<dyn std::error::Error>> {
    let game_state = load_game_state(config_path)?;
    let db = open_database(server_session).await?;
    Ok((game_state, db))
}

fn load_game_state(
    config_path: Option<&Path>,
) -> Result<game::GameState, Box<dyn std::error::Error>> {
    let state = game::GameState::load(config_path)?;
    tracing::info!(
        "Game state loaded: {} attributes",
        state.attribute_config.attributes.len()
    );
    Ok(state)
}

async fn open_database(
    server_session: &network::session::ServerSession,
) -> Result<persistence::Database, Box<dyn std::error::Error>> {
    let server_key = server_session.name.as_deref().unwrap_or("unnamed");
    let db = persistence::Database::connect(server_key).await?;
    tracing::info!("Database connected");
    Ok(db)
}

async fn serve_and_wait(
    server_session: network::session::ServerSession,
    game_state: game::GameState,
    db: persistence::Database,
) -> Result<(), Box<dyn std::error::Error>> {
    let session_name = server_session.name.clone();
    let addr = network::server::start(server_session, game_state, db.clone()).await?;
    network::discovery::start_discovery(addr.port(), session_name);
    tracing::info!("Server listening on {addr}");
    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn load_maps_into_db(
    db: &persistence::Database,
    config_path: Option<&Path>,
    forced: bool,
    game_state: &game::GameState,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Loading maps from config (forced={forced})");
    game::sync_universe_config(
        db.pool(),
        config_path,
        &game_state.faction_config,
        &game_state.resource_config,
    )
    .await
}
