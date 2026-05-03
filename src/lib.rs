pub mod cli;
pub mod game;
pub mod logging;
pub mod network;
pub mod persistence;
pub mod session;
pub mod state;
pub mod tui;

use cli::{Cli, Commands};

#[derive(Default)]
pub struct ServerConfig {}

pub async fn run_cli(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Some(Commands::Server {
            name,
            config,
            reload_maps,
        }) => run_server(name, config, reload_maps, ServerConfig::default()).await,
        Some(Commands::Client { url, debug }) => run_client(url, debug).await,
        None => run_client(None, false).await,
    }
}

pub async fn run_server(
    name: Option<String>,
    config: Option<String>,
    reload_maps: bool,
    _server_config: ServerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    logging::init_tracing();
    let (server_session, config_path_buf) = init_server_session(name, config).await?;
    let (game_state, db) = init_game_resources(&server_session, config_path_buf.as_deref()).await?;
    if reload_maps || game::should_auto_load(db.pool()).await? {
        load_maps_into_db(&db, config_path_buf.as_deref(), reload_maps).await?;
    }
    serve_and_wait(server_session, game_state, db, config_path_buf).await
}

async fn init_server_session(
    name: Option<String>,
    config: Option<String>,
) -> Result<(session::ServerSession, Option<std::path::PathBuf>), Box<dyn std::error::Error>> {
    let server_session = create_and_load_session(name).await?;
    let config_path_buf = find_config_path(config);
    Ok((server_session, config_path_buf))
}

async fn create_and_load_session(
    name: Option<String>,
) -> Result<session::ServerSession, Box<dyn std::error::Error>> {
    state::config::create_session_base_dirs().await?;
    let session = session::ServerSession::load_or_create(name).await?;
    tracing::info!("Server session loaded: {} {:?}", session.id, session.name);
    Ok(session)
}

fn find_config_path(config: Option<String>) -> Option<std::path::PathBuf> {
    let path = config
        .as_deref()
        .map(std::path::PathBuf::from)
        .or_else(state::config::find_config_dir);
    tracing::info!("Config directory resolved: {:?}", path);
    path
}

async fn init_game_resources(
    server_session: &session::ServerSession,
    config_path: Option<&std::path::Path>,
) -> Result<(game::GameState, persistence::Database), Box<dyn std::error::Error>> {
    let game_state = load_game_state(config_path)?;
    let db = open_database(server_session).await?;
    Ok((game_state, db))
}

fn load_game_state(
    config_path: Option<&std::path::Path>,
) -> Result<game::GameState, Box<dyn std::error::Error>> {
    let state = game::GameState::load(config_path)?;
    tracing::info!(
        "Game state loaded: {} attributes",
        state.attribute_config.attributes.len()
    );
    Ok(state)
}

async fn open_database(
    server_session: &session::ServerSession,
) -> Result<persistence::Database, Box<dyn std::error::Error>> {
    let server_key = server_session.name.as_deref().unwrap_or("unnamed");
    let db = persistence::Database::connect(server_key).await?;
    tracing::info!("Database connected");
    Ok(db)
}

async fn serve_and_wait(
    server_session: session::ServerSession,
    game_state: game::GameState,
    db: persistence::Database,
    config_path_buf: Option<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let session_name = server_session.name.clone();
    let addr =
        network::server::start(server_session, game_state, db.clone(), config_path_buf).await?;
    network::discovery::start_discovery(addr.port(), session_name);
    tracing::info!("Server listening on {addr}");
    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn load_maps_into_db(
    db: &persistence::Database,
    config_path: Option<&std::path::Path>,
    forced: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Loading maps from config (forced={forced})");
    let universe = persist_universe(db, config_path).await?;
    if let Some(config_dir) = config_path {
        load_entity_data(db, &universe, config_dir).await?;
    }
    Ok(())
}

async fn persist_universe(
    db: &persistence::Database,
    config_path: Option<&std::path::Path>,
) -> Result<game::Universe, Box<dyn std::error::Error>> {
    let universe = game::load_map(config_path)?;
    game::load_map_into_db(db.pool(), &universe).await?;
    tracing::info!("Maps loaded into database");
    Ok(universe)
}

async fn load_entity_data(
    db: &persistence::Database,
    universe: &game::Universe,
    config_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let entity_configs = game::load_entity_configs(config_dir)?;
    game::load_entities_into_db(db.pool(), universe, &entity_configs).await?;
    tracing::info!("Entities loaded into database");
    Ok(())
}

pub async fn run_client(
    url: Option<String>,
    debug: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match url {
        Some(url) => tui::run_client(Some(url), debug).await,
        None => tui::run_discovery(debug).await,
    }
}
