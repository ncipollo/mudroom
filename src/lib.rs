pub mod cli;
pub mod game;
pub mod network;
pub mod persistence;
pub mod session;
pub mod state;
pub mod tui;

use cli::{Cli, Commands};

pub async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Some(Commands::Server {
            name,
            config,
            reload_maps,
        }) => run_server(name, config, reload_maps).await,
        Some(Commands::Client {
            url: Some(url),
            debug,
        }) => tui::run_client(Some(url), debug).await,
        None | Some(Commands::Client { url: None, .. }) => run_discovery().await,
    }
}

async fn run_server(
    name: Option<String>,
    config: Option<String>,
    reload_maps: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    state::config::create_session_base_dirs().await?;
    let server_session = session::ServerSession::load_or_create(name).await?;
    tracing::info!(id = %server_session.id, name = ?server_session.name, "Server session loaded");

    let config_path_buf = config
        .as_deref()
        .map(std::path::PathBuf::from)
        .or_else(find_config_dir);
    tracing::info!(config_dir = ?config_path_buf, "Config directory resolved");

    let game_state = load_game_state(&config_path_buf)?;
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
    start_discovery(addr.port(), session_name);
    tracing::info!("Server listening on {addr}");
    tokio::signal::ctrl_c().await?;
    Ok(())
}

fn find_config_dir() -> Option<std::path::PathBuf> {
    let cwd = std::env::current_dir().ok()?;

    // Check working directory itself
    if cwd.join("mud.toml").exists() {
        return Some(cwd);
    }

    // Check immediate subdirectories of muds/
    let muds_dir = cwd.join("muds");
    if muds_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&muds_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("mud.toml").exists() {
                return Some(path);
            }
        }
    }

    None
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn load_game_state(
    config_dir: &Option<std::path::PathBuf>,
) -> Result<game::GameState, Box<dyn std::error::Error>> {
    let game_state = game::GameState::load(config_dir.as_deref())?;
    tracing::info!(
        attribute_count = game_state.attribute_config.attributes.len(),
        config_dir = ?config_dir,
        "Game state loaded"
    );
    Ok(game_state)
}

fn start_discovery(port: u16, session_name: Option<String>) {
    let discovery = network::discovery::DiscoveryServer::new(port, session_name);
    tokio::spawn(async move {
        let _ = discovery.run().await;
    });
}

async fn run_discovery() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
    let selected = tui::run_discovery(&mut terminal).await;
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    ratatui::restore();
    match selected? {
        Some(url) => tui::run_client(Some(url), false).await,
        None => Ok(()),
    }
}
