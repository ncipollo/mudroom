use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers, MouseEventKind};
use futures_util::StreamExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;
use tokio::time;

use crate::game::{Interaction, Movement, TurnAction};
use crate::network::NetworkEvent;
use crate::network::client::send_interaction;
use crate::network::client::{create_player, list_players, select_player};

use super::app::{App, AppMessage, GameMode};
use super::commands;
use super::screens::game as layout;

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
        terminal.draw(|frame| layout::render(frame, app))?;

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
                    handle_player_select_key(app, key.modifiers, key.code).await;
                }
                GameMode::Game => {
                    handle_game_key(app, key.modifiers, key.code).await;
                }
                GameMode::StandardConversation => {
                    handle_conversation_key(app, key.modifiers, key.code).await;
                }
                GameMode::AgentConversation => {
                    handle_agent_conversation_key(app, key.modifiers, key.code).await;
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
                    let player_id = info.id;
                    app.player_select.players.push(info);
                    app.cancel_create();
                    if select_player(&url, &client_id, player_id).await.is_ok() {
                        app.mode = GameMode::Game;
                    }
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
                    app.mode = GameMode::Game;
                }
            }
        }
        _ => {}
    }
}

async fn handle_conversation_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    match (modifiers, code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            app.should_quit = true;
        }
        (_, KeyCode::Up) => app.conversation.select_prev(),
        (_, KeyCode::Down) => app.conversation.select_next(),
        (_, KeyCode::Enter) => {
            if let Some(choice) = app.conversation.selected_choice()
                && let (Some(url), Some(client_id)) = (
                    app.connection.server_url.as_deref(),
                    app.connection.client_id.as_deref(),
                )
            {
                let action =
                    Interaction::EngagementAction(TurnAction::SelectDialogChoice { choice });
                let _ = send_interaction(url, client_id, &action).await;
            }
        }
        (_, KeyCode::PageUp) => app.scroll_up(),
        (_, KeyCode::PageDown) => app.scroll_down(),
        _ => {}
    }
}

async fn handle_agent_conversation_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
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
            let input: String = app.input.drain(..).collect();
            if input.trim() == "/exit" {
                if let (Some(url), Some(client_id)) = (
                    app.connection.server_url.as_deref(),
                    app.connection.client_id.as_deref(),
                ) {
                    let _ = send_interaction(url, client_id, &Interaction::EndConversation).await;
                }
            } else if !input.is_empty() {
                if let (Some(url), Some(client_id)) = (
                    app.connection.server_url.as_deref(),
                    app.connection.client_id.as_deref(),
                ) {
                    let action = Interaction::EngagementAction(TurnAction::Respond {
                        content: input.clone(),
                    });
                    let _ = send_interaction(url, client_id, &action).await;
                }
                app.messages.push(AppMessage::normal(input));
                app.scroll_offset = 0;
                app.agent_responding = true;
            }
        }
        (_, KeyCode::PageUp) => app.scroll_up(),
        (_, KeyCode::PageDown) => app.scroll_down(),
        _ => {}
    }
}

async fn handle_game_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
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
            let input: String = app.input.drain(..).collect();
            let cmd = commands::parse(&input);
            let url = app.connection.server_url.as_deref();
            let client_id = app.connection.client_id.as_deref();
            match cmd {
                commands::Command::Move(direction) => {
                    if let (Some(url), Some(client_id)) = (url, client_id) {
                        let interaction = Interaction::Movement(Movement::TryDirection(direction));
                        let _ = send_interaction(url, client_id, &interaction).await;
                    }
                }
                commands::Command::Look => {
                    if let (Some(url), Some(client_id)) = (url, client_id) {
                        let _ = send_interaction(url, client_id, &Interaction::Look).await;
                    }
                }
                commands::Command::Help => {
                    if let (Some(url), Some(client_id)) = (url, client_id) {
                        let _ = send_interaction(url, client_id, &Interaction::Help).await;
                    }
                }
                commands::Command::Talk => {
                    if let (Some(url), Some(client_id)) = (url, client_id) {
                        let _ =
                            send_interaction(url, client_id, &Interaction::StartConversation).await;
                    }
                }
                commands::Command::Choose(choice) => {
                    if let (Some(url), Some(client_id)) = (url, client_id) {
                        let action =
                            Interaction::EngagementAction(TurnAction::SelectDialogChoice {
                                choice,
                            });
                        let _ = send_interaction(url, client_id, &action).await;
                    }
                }
                _ => {}
            }
            app.messages.push(AppMessage::normal(input));
            app.scroll_offset = 0;
        }
        (_, KeyCode::PageUp) => app.scroll_up(),
        (_, KeyCode::PageDown) => app.scroll_down(),
        _ => {}
    }
}
