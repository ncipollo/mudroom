use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers, MouseEventKind};
use futures_util::StreamExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

use crate::network::NetworkEvent;
use crate::network::client::{create_player, list_players, select_player};

use super::app::{App, AppMode};
use super::layout;

pub async fn run(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    mut net_rx: mpsc::Receiver<NetworkEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut event_stream = EventStream::new();

    // Load player list immediately if in PlayerSelect mode
    if app.mode == AppMode::PlayerSelect
        && let (Some(url), Some(client_id)) = (
            app.connection.server_url.clone(),
            app.connection.client_id.clone(),
        )
        && let Ok(players) = list_players(&url, &client_id).await
    {
        app.player_select.players = players;
    }

    while !app.should_quit {
        terminal.draw(|frame| layout::render(frame, app))?;

        tokio::select! {
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => {
                        match app.mode {
                            AppMode::PlayerSelect => {
                                handle_player_select_key(app, key.modifiers, key.code).await;
                            }
                            AppMode::Game => {
                                handle_game_key(app, key.modifiers, key.code);
                            }
                        }
                    }
                    Some(Ok(Event::Mouse(mouse))) => match mouse.kind {
                        MouseEventKind::ScrollUp => app.scroll_up(),
                        MouseEventKind::ScrollDown => app.scroll_down(),
                        _ => {}
                    },
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
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

async fn handle_player_select_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    if modifiers == KeyModifiers::CONTROL && code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    if app.player_select.creating_player {
        match code {
            KeyCode::Esc => app.cancel_create(),
            KeyCode::Backspace => {
                app.player_select.player_name_input.pop();
            }
            KeyCode::Enter => {
                let name = app.player_select.player_name_input.trim().to_string();
                if !name.is_empty()
                    && let (Some(url), Some(client_id)) = (
                        app.connection.server_url.clone(),
                        app.connection.client_id.clone(),
                    )
                    && let Ok(info) = create_player(&url, &client_id, &name).await
                {
                    app.player_select.players.push(info);
                    app.cancel_create();
                    app.mode = AppMode::Game;
                }
            }
            KeyCode::Char(c) => app.player_select.player_name_input.push(c),
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Up => app.select_prev(),
        KeyCode::Down => app.select_next(),
        KeyCode::Enter => {
            let create_idx = app.player_select.players.len();
            if app.player_select.selected_index == create_idx {
                app.start_create();
            } else if let Some(player) = app
                .player_select
                .players
                .get(app.player_select.selected_index)
            {
                let player_id = player.id;
                if let (Some(url), Some(client_id)) = (
                    app.connection.server_url.clone(),
                    app.connection.client_id.clone(),
                ) && select_player(&url, &client_id, player_id).await.is_ok()
                {
                    app.mode = AppMode::Game;
                }
            }
        }
        _ => {}
    }
}

fn handle_game_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    match (modifiers, code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            app.should_quit = true;
        }
        (_, KeyCode::Char(c)) => {
            app.input.push(c);
        }
        (_, KeyCode::Backspace) => {
            app.input.pop();
        }
        (_, KeyCode::Enter) => {
            let msg = app.input.drain(..).collect();
            app.messages.push(msg);
            app.scroll_offset = 0;
        }
        _ => {}
    }
}
