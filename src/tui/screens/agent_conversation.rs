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
            app.command_history.reset_navigation();
        }
        (_, KeyCode::Backspace) => {
            app.input.pop();
            app.command_history.reset_navigation();
        }
        (_, KeyCode::Up) => {
            if let Some(recalled) = app.command_history.prev(&app.input) {
                app.input = recalled;
            }
        }
        (_, KeyCode::Down) => {
            if let Some(recalled) = app.command_history.next() {
                app.input = recalled;
            }
        }
        (_, KeyCode::Enter) => {
            app.skip_all_reveals();
            let input: String = std::mem::take(&mut app.input);
            if input.is_empty() {
                return;
            }
            app.command_history.push(input.clone());
            if input.trim() == "/exit" {
                app.send_interaction_async(Interaction::EndConversation);
            } else {
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn press(app: &mut App, code: KeyCode) {
        handle_key(app, KeyModifiers::NONE, code).await;
    }

    async fn type_str(app: &mut App, s: &str) {
        for c in s.chars() {
            press(app, KeyCode::Char(c)).await;
        }
    }

    #[tokio::test]
    async fn up_recalls_last_message() {
        let mut app = App::new(false);
        type_str(&mut app, "hello there").await;
        press(&mut app, KeyCode::Enter).await;
        press(&mut app, KeyCode::Up).await;
        assert_eq!(app.input, "hello there");
    }

    #[tokio::test]
    async fn up_recalls_slash_exit_too() {
        let mut app = App::new(false);
        type_str(&mut app, "/exit").await;
        press(&mut app, KeyCode::Enter).await;
        press(&mut app, KeyCode::Up).await;
        assert_eq!(app.input, "/exit");
    }

    #[tokio::test]
    async fn down_restores_draft() {
        let mut app = App::new(false);
        type_str(&mut app, "hello").await;
        press(&mut app, KeyCode::Enter).await;
        type_str(&mut app, "partial").await;
        press(&mut app, KeyCode::Up).await;
        press(&mut app, KeyCode::Down).await;
        assert_eq!(app.input, "partial");
    }

    #[tokio::test]
    async fn editing_after_recall_exits_navigation() {
        let mut app = App::new(false);
        type_str(&mut app, "hello").await;
        press(&mut app, KeyCode::Enter).await;
        press(&mut app, KeyCode::Up).await;
        press(&mut app, KeyCode::Char('!')).await;
        press(&mut app, KeyCode::Down).await;
        assert_eq!(app.input, "hello!");
    }
}
