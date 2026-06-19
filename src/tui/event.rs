use std::time::Duration;

use crossterm::event::{Event, EventStream, MouseEventKind};
use futures_util::StreamExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;
use tokio::time;

use crate::network::NetworkEvent;
use crate::network::client::list_players;

use super::app::{App, GameMode};
use super::screens::{
    agent_conversation, battle as battle_screen, conversation, game as game_screen, player_select,
};

pub async fn run(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    mut net_rx: mpsc::Receiver<NetworkEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut event_stream = EventStream::new();
    let mut spinner_ticker = time::interval(Duration::from_millis(100));
    spinner_ticker.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    // Load player list immediately if in PlayerSelect mode
    if app.mode == GameMode::PlayerSelect
        && let (Some(url), Some(client_id)) = (
            app.connection.server_url.clone(),
            app.connection.client_id.clone(),
        )
        && let Ok(players) = list_players(&url, &client_id).await
    {
        app.player_select.players = players;
    }

    while !app.should_quit {
        terminal.draw(|frame| game_screen::render(frame, app))?;

        tokio::select! {
            _ = spinner_ticker.tick(), if app.agent_responding => {}
            maybe_event = event_stream.next() => {
                if !handle_terminal_event(app, maybe_event).await {
                    break;
                }
            }
            maybe_net = net_rx.recv() => {
                if let Some(event) = maybe_net {
                    app.handle_network_event(event);
                }
            }
        }
    }

    Ok(())
}

/// Dispatches a terminal event. Returns `false` if the event loop should exit.
async fn handle_terminal_event(
    app: &mut App,
    maybe_event: Option<Result<Event, std::io::Error>>,
) -> bool {
    match maybe_event {
        Some(Ok(Event::Key(key))) => {
            match app.mode {
                GameMode::PlayerSelect => {
                    player_select::handle_key(app, key.modifiers, key.code).await;
                }
                GameMode::Game => {
                    game_screen::handle_key(app, key.modifiers, key.code).await;
                }
                GameMode::StandardConversation => {
                    conversation::handle_key(app, key.modifiers, key.code).await;
                }
                GameMode::AgentConversation => {
                    agent_conversation::handle_key(app, key.modifiers, key.code).await;
                }
                GameMode::Battle => {
                    battle_screen::handle_key(app, key.modifiers, key.code).await;
                }
            }
            true
        }
        Some(Ok(Event::Mouse(mouse))) => {
            match mouse.kind {
                MouseEventKind::ScrollUp => app.scroll_up(),
                MouseEventKind::ScrollDown => app.scroll_down(),
                _ => {}
            }
            true
        }
        Some(Ok(_)) => true,
        Some(Err(_)) | None => false,
    }
}
