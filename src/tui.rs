mod app;
mod commands;
mod components;
mod event;
mod screens;

pub use app::App;

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{network, paths};

pub async fn run_client(
    url: Option<String>,
    debug: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let (net_tx, net_rx) = mpsc::channel(64);

    // Track session info for cleanup on exit: (session, server_url, connection_key).
    let mut client_session_info: Option<(network::session::ClientSession, String, String)> = None;

    if let Some(ref server_url) = url {
        paths::create_session_base_dirs().await?;

        // Fetch server identity without creating a session.
        let server_info = network::client::get_server_info(server_url).await?;
        let server_id = server_info.server_id;

        // Reuse the saved machine UUID, or generate a fresh one.
        let saved_uuid = network::session::ClientSession::load(&server_id)
            .await
            .ok()
            .flatten()
            .map(|s| s.id);
        let machine_uuid = saved_uuid.unwrap_or_else(|| Uuid::new_v4().to_string());
        let client_session = network::session::ClientSession {
            id: machine_uuid,
            name: None,
        };

        // Per-process connection key: uuid:pid — unique across concurrent clients.
        let connection_key = client_session.connection_key();
        network::client::start_session(server_url, connection_key.clone()).await?;
        client_session.save(&server_id).await?;

        // Spawn SSE listener.
        let sse_url = server_url.clone();
        let sse_key = connection_key.clone();
        let sse_tx = net_tx.clone();
        tokio::spawn(async move {
            network::client::connect_sse(sse_url, sse_key, sse_tx)
                .await
                .ok();
        });

        // Spawn periodic ping loop.
        let ping_url = server_url.clone();
        let ping_key = connection_key.clone();
        tokio::spawn(async move {
            network::client::run_ping_loop(ping_url, ping_key)
                .await
                .ok();
        });

        client_session_info = Some((client_session, server_url.clone(), connection_key));
    }

    let mut app = if let Some((_, ref server_url, ref connection_key)) = client_session_info {
        App::with_player_select(server_url.clone(), connection_key.clone(), debug)
    } else {
        App::new(debug)
    };
    let mut terminal = ratatui::init();
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;

    let result = event::run(&mut terminal, &mut app, net_rx).await;

    // Send EndSession on exit.
    if let Some((_, server_url, connection_key)) = client_session_info {
        network::client::end_session(&server_url, &connection_key)
            .await
            .ok();
    }

    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    ratatui::restore();
    result
}

pub async fn run_discovery(debug: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
    let selected = screens::discovery::run(&mut terminal).await;
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    ratatui::restore();
    match selected? {
        Some(url) => run_client(Some(url), debug).await,
        None => Ok(()),
    }
}
