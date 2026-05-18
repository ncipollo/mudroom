use std::time::SystemTime;

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::app::App;

const SPINNER_FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

fn spinner_frame() -> char {
    let idx = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() / 100)
        .unwrap_or(0) as usize;
    SPINNER_FRAMES[idx % SPINNER_FRAMES.len()]
}

pub fn render(frame: &mut Frame, app: &App) {
    let areas = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(3),
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
        .block(Block::default().title("Conversation").borders(Borders::ALL));
    frame.render_widget(log, areas[0]);

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

    let hint = Paragraph::new("/exit  Leave conversation  •  PgUp/PgDn  Scroll")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(hint, areas[3]);
}
