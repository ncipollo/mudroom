use ratatui::{Frame, layout::Rect, text::Line, widgets::Block};

use crate::tui::app::App;
use crate::tui::components::{scroll, typewriter};

pub fn render(frame: &mut Frame, app: &mut App, block: Block, area: Rect) {
    // Messages still waiting their turn in the reveal queue haven't "started"
    // yet and stay hidden, rather than flashing in full before their
    // typewriter reveal begins.
    let all_lines: Vec<Line> = app
        .messages
        .iter()
        .enumerate()
        .flat_map(|(i, msg)| match &app.reveal {
            Some(state) if state.message_index == i => {
                typewriter::truncate_lines(&msg.lines, state.revealed_chars)
            }
            _ if app.reveal_queue.contains(&i) => Vec::new(),
            _ => msg.lines.clone(),
        })
        .collect();

    scroll::render_scrolled_lines(frame, all_lines, &mut app.log_scroll, block, area, true);
}
