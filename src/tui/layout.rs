use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
};

use super::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let areas = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(8),
        Constraint::Length(3),
    ])
    .split(frame.area());

    // Game world panel
    let world =
        Paragraph::new("Game world").block(Block::default().title("World").borders(Borders::ALL));
    frame.render_widget(world, areas[0]);

    // Status bar
    let status = Paragraph::new("HP: 100 | MP: 50 | Location: Town Square")
        .style(Style::default().fg(Color::Green))
        .block(Block::default().title("Status").borders(Borders::ALL));
    frame.render_widget(status, areas[1]);

    // Message log
    let visible_lines = areas[2].height.saturating_sub(2) as usize;
    let total = app.messages.len();
    let max_offset = total.saturating_sub(visible_lines);
    let effective_offset = app.scroll_offset.min(max_offset);
    let end = total.saturating_sub(effective_offset);
    let start = end.saturating_sub(visible_lines);
    let log_text = app.messages[start..end].join("\n");
    let log =
        Paragraph::new(log_text).block(Block::default().title("Messages").borders(Borders::ALL));
    frame.render_widget(log, areas[2]);

    // Input line
    let input_text = format!("> {}", app.input);
    let input = Paragraph::new(Text::from(input_text))
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().title("Input").borders(Borders::ALL));
    frame.render_widget(input, areas[3]);
}
