use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use super::super::components::message_log;
use crate::game::{Interaction, TurnAction};
use crate::network::client::send_interaction;
use crate::tui::app::App;

pub async fn handle_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
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
        _ => {}
    }
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let option_count = app.conversation.options.len() as u16;
    let areas = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(option_count + 2),
        Constraint::Length(3),
    ])
    .split(frame.area());

    message_log::render(
        frame,
        app,
        Block::default().title("Messages").borders(Borders::ALL),
        areas[0],
    );

    let items: Vec<ListItem> = app
        .conversation
        .options
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let label = format!("[{}] {}", i + 1, text);
            if i == app.conversation.selected_index {
                ListItem::new(label).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ListItem::new(label)
            }
        })
        .collect();
    let list = List::new(items).block(Block::default().title("Options").borders(Borders::ALL));
    frame.render_widget(list, areas[1]);

    let hint = Paragraph::new("↑↓ Navigate  •  Enter Confirm")
        .style(Style::default().fg(Color::Green))
        .block(Block::default().title("Controls").borders(Borders::ALL));
    frame.render_widget(hint, areas[2]);
}
