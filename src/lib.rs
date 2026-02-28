pub mod cli;
pub mod game;
pub mod network;
pub mod session;
pub mod state;
pub mod tui;

use cli::{Cli, Commands};

pub async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Some(Commands::Server { name }) => {
            let filter = tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
            tracing_subscriber::fmt().with_env_filter(filter).init();
            state::config::create_session_base_dirs().await?;
            let server_session = session::ServerSession::load_or_create(name).await?;
            tracing::info!(id = %server_session.id, name = ?server_session.name, "Server session loaded");
            let session_name = server_session.name.clone();
            let addr = network::server::start(server_session).await?;
            let discovery = network::discovery::DiscoveryServer::new(addr.port(), session_name);
            tokio::spawn(async move {
                let _ = discovery.run().await;
            });
            tracing::info!("Server listening on {addr}");
            tokio::signal::ctrl_c().await?;
            Ok(())
        }
        Some(Commands::Client { url: Some(url) }) => tui::run_client(Some(url)).await,
        None | Some(Commands::Client { url: None }) => {
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
    }
}
