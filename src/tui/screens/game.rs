use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
};

use super::super::components::message_log;
use super::{agent_conversation, conversation, player_select};
use crate::tui::app::{App, GameMode};

pub fn render(frame: &mut Frame, app: &App) {
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

    let areas = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(3),
    ])
    .split(frame.area());

    // Message log
    message_log::render(
        frame,
        &app.messages,
        app.scroll_offset,
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
