use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use super::super::components::message_log;
use crate::tui::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let option_count = app.conversation.options.len() as u16;
    let areas = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(option_count + 2),
        Constraint::Length(3),
    ])
    .split(frame.area());

    message_log::render(
        frame,
        &app.messages,
        app.scroll_offset,
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
