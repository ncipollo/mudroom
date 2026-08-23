use std::time::SystemTime;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
};

use super::super::components::{cursor, message_log};
use crate::game::{Interaction, TurnAction};
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
            app.skip_all_reveals();
            let input: String = std::mem::take(&mut app.input);
            if input.trim() == "/exit" {
                app.send_interaction_async(Interaction::EndConversation);
            } else if !input.is_empty() {
                let action = Interaction::EngagementAction(TurnAction::Respond {
                    content: input.clone(),
                });
                app.send_interaction_async(action);
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
    let status =
        Paragraph::new(status_text).block(Block::default().title("Status").borders(Borders::ALL));
    frame.render_widget(status, areas[1]);

    let input_block = Block::default().title("Message").borders(Borders::ALL);
    let input_inner = input_block.inner(areas[2]);
    let input_text = format!("> {}", app.input);
    let input = Paragraph::new(Text::from(input_text)).block(input_block);
    frame.render_widget(input, areas[2]);
    cursor::place_at_end(frame, input_inner, 2, &app.input);

    let hint = Paragraph::new("/exit  Leave conversation  •  PgUp/PgDn Page  •  Shift+↑↓ Scroll")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(hint, areas[3]);
}
