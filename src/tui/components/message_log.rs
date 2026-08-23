mod layout;

pub use layout::LogLayout;

use ratatui::{Frame, layout::Rect, text::Line, widgets::Block};

use crate::tui::app::{App, AppMessage};
use crate::tui::components::{scroll, typewriter};
use layout::wrapped_row_count;

/// Renders the bottom-pinned message log, materializing only the messages that intersect the
/// visible viewport (plus whatever's needed to keep the scroll offset exact) instead of the
/// full history, so per-frame cost stays independent of how long the session has run.
pub fn render(frame: &mut Frame, app: &mut App, block: Block, area: Rect) {
    let inner = block.inner(area);
    let width = inner.width;
    let viewport_rows = inner.height as usize;

    app.log_layout
        .sync(&app.messages, app.streaming_message_index, width);

    let overrides = reveal_overrides(app, width);
    let total_rows = effective_total_rows(app, &overrides);
    app.log_scroll.sync(total_rows, viewport_rows);

    let (lines, rows_before_start) = visible_window(app, &overrides, total_rows, viewport_rows);

    let scroll_from_top = total_rows
        .saturating_sub(viewport_rows)
        .saturating_sub(app.log_scroll.offset())
        .saturating_sub(rows_before_start);

    scroll::render_slice(frame, lines, scroll_from_top, block, area, true);
}

/// Row-count overrides for messages whose displayed content differs from their full, settled
/// form: the actively-revealing message (truncated to what's been typed out so far) and any
/// narration messages still waiting in the reveal queue (not shown at all yet).
fn reveal_overrides(app: &App, width: u16) -> Vec<(usize, usize)> {
    let mut overrides: Vec<(usize, usize)> = app.reveal_queue.iter().map(|&idx| (idx, 0)).collect();
    if let Some(reveal) = &app.reveal
        && let Some(msg) = app.messages.get(reveal.message_index)
    {
        let truncated = typewriter::truncate_lines(&msg.lines, reveal.revealed_chars);
        overrides.push((reveal.message_index, wrapped_row_count(&truncated, width)));
    }
    overrides
}

fn effective_total_rows(app: &App, overrides: &[(usize, usize)]) -> usize {
    overrides
        .iter()
        .fold(app.log_layout.total_rows(), |total, &(idx, effective)| {
            total - app.log_layout.row_count(idx) + effective
        })
}

fn effective_row_count(app: &App, index: usize, overrides: &[(usize, usize)]) -> usize {
    overrides
        .iter()
        .find(|&&(idx, _)| idx == index)
        .map_or_else(|| app.log_layout.row_count(index), |&(_, rows)| rows)
}

/// Walks backward from the last message, accumulating rows (with `overrides` applied) until
/// enough content is gathered to cover the visible viewport plus the current scroll offset, then
/// materializes just that slice. Returns the slice's lines and the row count hidden above it, so
/// the caller can translate a full-log scroll offset into one relative to the slice.
fn visible_window(
    app: &App,
    overrides: &[(usize, usize)],
    total_rows: usize,
    viewport_rows: usize,
) -> (Vec<Line<'static>>, usize) {
    let needed = app.log_scroll.offset() + viewport_rows;
    let mut accumulated = 0usize;
    let mut start_index = app.messages.len();
    while start_index > 0 && accumulated < needed {
        start_index -= 1;
        accumulated += effective_row_count(app, start_index, overrides);
    }
    let rows_before_start = total_rows.saturating_sub(accumulated);

    let lines = app.messages[start_index..]
        .iter()
        .enumerate()
        .flat_map(|(offset, msg)| render_message_lines(app, start_index + offset, msg))
        .collect();

    (lines, rows_before_start)
}

/// Messages still waiting their turn in the reveal queue haven't "started" yet and stay hidden,
/// rather than flashing in full before their typewriter reveal begins.
fn render_message_lines(app: &App, index: usize, msg: &AppMessage) -> Vec<Line<'static>> {
    match &app.reveal {
        Some(state) if state.message_index == index => {
            typewriter::truncate_lines(&msg.lines, state.revealed_chars)
        }
        _ if app.reveal_queue.contains(&index) => Vec::new(),
        _ => msg.lines.clone(),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;

    use super::*;
    use crate::tui::components::theme::MessageTheme;

    fn app_with_messages(count: usize) -> App {
        let mut app = App::new(false);
        app.messages.clear();
        let theme = MessageTheme;
        for i in 0..count {
            app.messages
                .push(AppMessage::normal(format!("line {i}"), &theme));
        }
        app
    }

    fn render_buffer(app: &mut App, width: u16, height: u16) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render(frame, app, Block::default(), area);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn line_text(buffer: &Buffer, row: u16, width: u16) -> String {
        (0..width)
            .map(|x| buffer[(x, row)].symbol())
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn pinned_to_bottom_shows_the_most_recent_messages() {
        let mut app = app_with_messages(50);
        let buffer = render_buffer(&mut app, 20, 5);
        assert_eq!(line_text(&buffer, 0, 20), "line 45");
        assert_eq!(line_text(&buffer, 4, 20), "line 49");
    }

    #[test]
    fn scrolling_up_reveals_earlier_messages_at_the_exact_offset() {
        let mut app = app_with_messages(50);
        render_buffer(&mut app, 20, 5); // first render syncs the scroll state
        app.log_scroll.scroll_up(5);
        let buffer = render_buffer(&mut app, 20, 5);
        assert_eq!(line_text(&buffer, 0, 20), "line 40");
        assert_eq!(line_text(&buffer, 4, 20), "line 44");
    }

    #[test]
    fn scrolling_to_the_very_top_shows_the_first_message() {
        let mut app = app_with_messages(50);
        render_buffer(&mut app, 20, 5);
        app.log_scroll.scroll_up(1000);
        let buffer = render_buffer(&mut app, 20, 5);
        assert_eq!(line_text(&buffer, 0, 20), "line 0");
        assert_eq!(line_text(&buffer, 4, 20), "line 4");
    }

    #[test]
    fn total_row_count_tracks_message_count() {
        let mut app = app_with_messages(10);
        render_buffer(&mut app, 20, 5);
        assert_eq!(app.log_layout.total_rows(), 10);
    }

    #[test]
    fn short_history_renders_without_underflow() {
        let mut app = app_with_messages(2);
        let buffer = render_buffer(&mut app, 20, 5);
        assert_eq!(line_text(&buffer, 0, 20), "line 0");
        assert_eq!(line_text(&buffer, 1, 20), "line 1");
        assert_eq!(line_text(&buffer, 2, 20), "");
    }

    #[test]
    fn active_reveal_truncates_only_the_revealing_message() {
        let mut app = app_with_messages(3);
        app.start_reveal(2);
        if let Some(reveal) = &mut app.reveal {
            reveal.revealed_chars = 4;
        }
        let buffer = render_buffer(&mut app, 20, 5);
        assert_eq!(line_text(&buffer, 0, 20), "line 0");
        assert_eq!(line_text(&buffer, 1, 20), "line 1");
        assert_eq!(line_text(&buffer, 2, 20), "line");
    }
}
