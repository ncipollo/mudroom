mod app;
mod discovery;
mod event;
mod layout;

pub use app::App;

use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

use crate::network;

pub async fn run_client(url: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut terminal = ratatui::init();
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;

    let (net_tx, net_rx) = mpsc::channel(64);

    if let Some(ref u) = url {
        let u = u.clone();
        let tx = net_tx.clone();
        tokio::spawn(async move {
            network::client::connect_sse(u, tx).await.ok();
        });

        let ping_url = url.clone().unwrap();
        tokio::spawn(async move {
            network::client::send_ping(&ping_url).await.ok();
        });
    }

    let result = event::run(&mut terminal, &mut app, net_rx).await;
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    ratatui::restore();
    result
}

pub async fn run_discovery(
    terminal: &mut DefaultTerminal,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    discovery::run(terminal).await
}
