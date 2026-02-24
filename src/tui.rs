mod app;
mod discovery;
mod event;
mod layout;

pub use app::App;

use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

use crate::{network, session, state};

pub async fn run_client(url: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let (net_tx, net_rx) = mpsc::channel(64);

    // Track session info for cleanup on exit.
    let mut client_session_info: Option<(session::ClientSession, String)> = None;

    if let Some(ref server_url) = url {
        state::config::create_session_base_dirs().await?;

        // First call to get the server_id.
        let initial = network::client::start_session(server_url, None).await?;
        let server_id = initial.server_id.clone();

        // Try to reload a previously saved session for this server.
        let saved = session::ClientSession::load(&server_id)
            .await
            .ok()
            .flatten();
        let final_resp = if let Some(ref saved_session) = saved {
            network::client::start_session(server_url, Some(saved_session.id.clone())).await?
        } else {
            initial
        };

        let client_session = session::ClientSession {
            id: final_resp.client_id.clone(),
            name: None,
        };
        client_session.save(&server_id).await?;

        // Spawn SSE listener.
        let sse_url = server_url.clone();
        let sse_tx = net_tx.clone();
        tokio::spawn(async move {
            network::client::connect_sse(sse_url, sse_tx).await.ok();
        });

        // Spawn periodic ping loop.
        let ping_url = server_url.clone();
        let ping_client_id = client_session.id.clone();
        tokio::spawn(async move {
            network::client::run_ping_loop(ping_url, ping_client_id)
                .await
                .ok();
        });

        client_session_info = Some((client_session, server_url.clone()));
    }

    let mut app = App::new();
    let mut terminal = ratatui::init();
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;

    let result = event::run(&mut terminal, &mut app, net_rx).await;

    // Send EndSession on exit.
    if let Some((client_session, server_url)) = client_session_info {
        network::client::end_session(&server_url, &client_session.id)
            .await
            .ok();
    }

    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    ratatui::restore();
    result
}

pub async fn run_discovery(
    terminal: &mut DefaultTerminal,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    discovery::run(terminal).await
}
