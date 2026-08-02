use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
};

use super::super::components::message_log;
use super::{agent_conversation, battle, conversation, player_select};
use crate::game::{Interaction, Movement, TurnAction};
use crate::network::client::send_interaction;
use crate::tui::app::{App, AppMessage, GameMode};
use crate::tui::commands;

pub async fn handle_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
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
            app.skip_all_reveals();
            let input: String = app.input.drain(..).collect();
            if input.is_empty() {
                return;
            }
            dispatch_command(app, &input).await;
            app.messages.push(AppMessage::command(input, &app.theme));
            app.log_scroll.pin_to_bottom();
        }
        _ => {}
    }
}

async fn dispatch_command(app: &mut App, input: &str) {
    let cmd = commands::parse(input);
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
        commands::Command::Talk(msg) => {
            let has_initial = msg.is_some();
            if let (Some(url), Some(client_id)) = (url, client_id) {
                let _ = send_interaction(
                    url,
                    client_id,
                    &Interaction::StartConversation {
                        initial_message: msg,
                    },
                )
                .await;
                if has_initial {
                    app.agent_responding = true;
                }
            }
        }
        commands::Command::Choose(choice) => {
            if let (Some(url), Some(client_id)) = (url, client_id) {
                let action =
                    Interaction::EngagementAction(TurnAction::SelectDialogChoice { choice });
                let _ = send_interaction(url, client_id, &action).await;
            }
        }
        commands::Command::Attack => {
            if let (Some(url), Some(client_id)) = (url, client_id) {
                let _ = send_interaction(
                    url,
                    client_id,
                    &Interaction::JoinBattle { engagement_id: 0 },
                )
                .await;
            }
        }
        _ => {}
    }
}

pub fn render(frame: &mut Frame, app: &mut App) {
    if app.mode == GameMode::PlayerSelect {
        player_select::render(frame, app);
        return;
    }

    if app.mode == GameMode::StandardConversation {
        conversation::render(frame, app);
        return;
    }

    if app.mode == GameMode::AgentConversation {
        agent_conversation::render(frame, app);
        return;
    }

    if app.mode == GameMode::Battle {
        battle::render(frame, app);
        return;
    }

    let areas = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(3),
    ])
    .split(frame.area());

    // Message log
    message_log::render(
        frame,
        app,
        Block::default().title("Messages").borders(Borders::ALL),
        areas[0],
    );

    // Status bar
    let status = Paragraph::new("HP: 100 | MP: 50 | Location: Town Square")
        .style(Style::default().fg(Color::Green))
        .block(Block::default().title("Status").borders(Borders::ALL));
    frame.render_widget(status, areas[1]);

    // Input line
    let input_text = format!("> {}", app.input);
    let input = Paragraph::new(Text::from(input_text))
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().title("Input").borders(Borders::ALL));
    frame.render_widget(input, areas[2]);
}
