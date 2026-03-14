use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

use super::app::{App, AppMode};
use super::player_select;

pub fn render(frame: &mut Frame, app: &App) {
    if app.mode == AppMode::PlayerSelect {
        player_select::render(frame, app);
        return;
    }

    let areas = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(3),
    ])
    .split(frame.area());

    // Message log
    let visible_lines = areas[0].height.saturating_sub(2) as usize;
    let total = app.messages.len();
    let max_offset = total.saturating_sub(visible_lines);
    let effective_offset = app.scroll_offset.min(max_offset);
    let end = total.saturating_sub(effective_offset);
    let start = end.saturating_sub(visible_lines);
    let log_lines: Vec<Line> = app.messages[start..end]
        .iter()
        .map(|msg| {
            let style = if msg.debug {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            Line::from(Span::styled(msg.text.clone(), style))
        })
        .collect();
    let log = Paragraph::new(Text::from(log_lines))
        .block(Block::default().title("Messages").borders(Borders::ALL));
    frame.render_widget(log, areas[0]);

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
