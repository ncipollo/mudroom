use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Wrap},
};

use super::theme::{MessageTheme, StyleKey};
use crate::tui::app::AppMessage;

pub fn render(
    frame: &mut Frame,
    messages: &[AppMessage],
    theme: &MessageTheme,
    scroll_offset: usize,
    block: Block,
    area: Rect,
) {
    let panel_height = area.height.saturating_sub(2) as usize;

    let all_lines: Vec<Line> = messages
        .iter()
        .flat_map(|msg| {
            let style = theme.resolve(StyleKey::Message(msg.kind), None);
            msg.text
                .split('\n')
                .map(|line| Line::from(Span::styled(line.to_string(), style)))
                .collect::<Vec<_>>()
        })
        .collect();

    let total_lines = all_lines.len();
    let max_offset = total_lines.saturating_sub(panel_height);
    let effective_offset = scroll_offset.min(max_offset);
    let scroll_from_top = max_offset.saturating_sub(effective_offset) as u16;

    let log = Paragraph::new(Text::from(all_lines))
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((scroll_from_top, 0));
    frame.render_widget(log, area);
}
