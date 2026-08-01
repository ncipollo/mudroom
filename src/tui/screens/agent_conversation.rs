use std::time::SystemTime;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
};

use super::super::components::message_log;
use crate::game::{Interaction, TurnAction};
use crate::network::client::send_interaction;
use crate::tui::app::{App, AppMessage};

const SPINNER_FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

fn spinner_frame() -> char {
    let idx = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() / 100)
        .unwrap_or(0) as usize;
    SPINNER_FRAMES[idx % SPINNER_FRAMES.len()]
}

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
                app.messages.push(AppMessage::command(input, &app.theme));
                app.log_scroll.pin_to_bottom();
                app.agent_responding = true;
            }
        }
        _ => {}
    }
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let areas = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
    ])
    .split(frame.area());

    message_log::render(
        frame,
        app,
        Block::default().title("Conversation").borders(Borders::ALL),
        areas[0],
    );

    let status_text = if app.agent_responding {
        format!("{} Responding...", spinner_frame())
    } else {
        "Your turn".to_string()
    };
    let status_style = if app.agent_responding {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };
    let status = Paragraph::new(status_text)
        .style(status_style)
        .block(Block::default().title("Status").borders(Borders::ALL));
    frame.render_widget(status, areas[1]);

    let input_text = format!("> {}", app.input);
    let input = Paragraph::new(Text::from(input_text))
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().title("Message").borders(Borders::ALL));
    frame.render_widget(input, areas[2]);

    let hint = Paragraph::new("/exit  Leave conversation  •  PgUp/PgDn Page  •  Shift+↑↓ Scroll")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(hint, areas[3]);
}
