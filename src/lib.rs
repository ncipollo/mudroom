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
        Some(Commands::Client { url: Some(url) }) => tui::run_client(Some(url)).await,
        None | Some(Commands::Client { url: None }) => run_discovery().await,
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
    let game_state = load_game_state(&config)?;
    let server_key = server_session.name.as_deref().unwrap_or("unnamed");
    let db = persistence::Database::connect(server_key).await?;
    tracing::info!("Database connected");

    let config_path = config.as_deref().map(std::path::Path::new);
    if reload_maps || game::should_auto_load(db.pool()).await? {
        tracing::info!(forced = reload_maps, "Loading maps from config");
        let universe = game::load_map(config_path)?;
        game::load_map_into_db(db.pool(), &universe).await?;
        tracing::info!("Maps loaded into database");
    }

    let session_name = server_session.name.clone();
    let addr = network::server::start(server_session, game_state, db.clone()).await?;
    start_discovery(addr.port(), session_name);
    tracing::info!("Server listening on {addr}");
    run_server_commands(db, config_path).await?;
    Ok(())
}

async fn run_server_commands(
    db: persistence::Database,
    config_path: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::io::AsyncBufReadExt;
    let stdin = tokio::io::stdin();
    let mut lines = tokio::io::BufReader::new(stdin).lines();
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            line = lines.next_line() => {
                match line? {
                    None => break,
                    Some(s) if s.trim().is_empty() => continue,
                    Some(s) if s.trim() == "reload-maps" => {
                        tracing::info!("Reloading maps");
                        let universe = game::load_map(config_path)?;
                        game::load_map_into_db(db.pool(), &universe).await?;
                        tracing::info!("Maps reloaded");
                    }
                    Some(s) if matches!(s.trim(), "quit" | "exit") => break,
                    Some(s) => tracing::warn!(command = %s.trim(), "Unknown server command"),
                }
            }
        }
    }
    Ok(())
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn load_game_state(config: &Option<String>) -> Result<game::GameState, Box<dyn std::error::Error>> {
    let config_path = config.as_deref().map(std::path::Path::new);
    let game_state = game::GameState::load(config_path)?;
    tracing::info!(
        attribute_count = game_state.attribute_config.attributes.len(),
        config_dir = ?config,
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
        Some(url) => tui::run_client(Some(url)).await,
        None => Ok(()),
    }
}
