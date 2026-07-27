use std::collections::VecDeque;

use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Text},
    widgets::{Block, Paragraph, Wrap},
};

use crate::tui::app::AppMessage;
use crate::tui::components::typewriter::{self, TypewriterState};

pub fn render(
    frame: &mut Frame,
    messages: &[AppMessage],
    scroll_offset: usize,
    block: Block,
    area: Rect,
    reveal: Option<&TypewriterState>,
    reveal_queue: &VecDeque<usize>,
) {
    let panel_height = area.height.saturating_sub(2) as usize;

    // Messages still waiting their turn in the reveal queue haven't "started"
    // yet and stay hidden, rather than flashing in full before their
    // typewriter reveal begins.
    let all_lines: Vec<Line> = messages
        .iter()
        .enumerate()
        .flat_map(|(i, msg)| match reveal {
            Some(state) if state.message_index == i => {
                typewriter::truncate_lines(&msg.lines, state.revealed_chars)
            }
            _ if reveal_queue.contains(&i) => Vec::new(),
            _ => msg.lines.clone(),
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
