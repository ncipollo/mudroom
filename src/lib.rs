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
    state::config::create_session_base_dirs().await?;
    let server_session = session::ServerSession::load_or_create(name).await?;
    tracing::info!(id = %server_session.id, name = ?server_session.name, "Server session loaded");

    let config_path_buf = config
        .as_deref()
        .map(std::path::PathBuf::from)
        .or_else(state::config::find_config_dir);
    tracing::info!(config_dir = ?config_path_buf, "Config directory resolved");

    let game_state = game::GameState::load(config_path_buf.as_deref())?;
    tracing::info!(
        attribute_count = game_state.attribute_config.attributes.len(),
        config_dir = ?config_path_buf,
        "Game state loaded"
    );

    let server_key = server_session.name.as_deref().unwrap_or("unnamed");
    let db = persistence::Database::connect(server_key).await?;
    tracing::info!("Database connected");

    let config_path = config_path_buf.as_deref();
    if reload_maps || game::should_auto_load(db.pool()).await? {
        tracing::info!(forced = reload_maps, "Loading maps from config");
        let universe = game::load_map(config_path)?;
        game::load_map_into_db(db.pool(), &universe).await?;
        tracing::info!("Maps loaded into database");
    }

    let session_name = server_session.name.clone();
    let addr =
        network::server::start(server_session, game_state, db.clone(), config_path_buf).await?;
    network::discovery::start_discovery(addr.port(), session_name);
    tracing::info!("Server listening on {addr}");
    tokio::signal::ctrl_c().await?;
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
