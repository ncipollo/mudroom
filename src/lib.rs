pub mod cli;
pub mod network;
pub mod tui;

use cli::{Cli, Commands};

pub async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Some(Commands::Server) => {
            let filter = tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
            tracing_subscriber::fmt().with_env_filter(filter).init();
            let addr = network::server::start().await?;
            let discovery = network::discovery::DiscoveryServer::new(addr.port());
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
