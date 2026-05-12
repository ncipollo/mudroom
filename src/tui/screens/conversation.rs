use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::tui::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let option_count = app.conversation.options.len() as u16;
    let areas = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(option_count + 2),
        Constraint::Length(3),
    ])
    .split(frame.area());

    let visible_lines = areas[0].height.saturating_sub(2) as usize;
    let total = app.messages.len();
    let max_offset = total.saturating_sub(visible_lines);
    let effective_offset = app.scroll_offset.min(max_offset);
    let end = total.saturating_sub(effective_offset);
    let start = end.saturating_sub(visible_lines);
    let log_lines: Vec<Line> = app.messages[start..end]
        .iter()
        .flat_map(|msg| {
            let style = if msg.debug {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            msg.text
                .split('\n')
                .map(|line| Line::from(Span::styled(line.to_string(), style)))
                .collect::<Vec<_>>()
        })
        .collect();
    let log = Paragraph::new(Text::from(log_lines))
        .block(Block::default().title("Messages").borders(Borders::ALL));
    frame.render_widget(log, areas[0]);

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
