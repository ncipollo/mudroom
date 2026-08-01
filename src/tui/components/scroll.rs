use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Text},
    widgets::{Block, Paragraph, Wrap},
};

/// Rows scrolled per mouse wheel event.
pub const MOUSE_SCROLL_LINES: usize = 3;
/// Rows scrolled per modifier+arrow key press.
pub const KEY_SCROLL_LINES: usize = 1;

/// Scroll position for a bottom-pinned log, measured in wrapped rows above
/// the bottom. An offset of 0 means pinned: the view follows new content.
///
/// `viewport_rows` and `max_offset` are refreshed by [`Self::sync`] each
/// frame, so scroll requests clamp against the previous frame's metrics and
/// are re-clamped on the next draw.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScrollState {
    offset: usize,
    viewport_rows: usize,
    max_offset: usize,
}

impl ScrollState {
    pub fn scroll_up(&mut self, n: usize) {
        self.offset = (self.offset + n).min(self.max_offset);
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.offset = self.offset.saturating_sub(n);
    }

    pub fn page_up(&mut self) {
        self.scroll_up(self.viewport_rows.max(1));
    }

    pub fn page_down(&mut self) {
        self.scroll_down(self.viewport_rows.max(1));
    }

    pub fn pin_to_bottom(&mut self) {
        self.offset = 0;
    }

    pub fn is_pinned(&self) -> bool {
        self.offset == 0
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Refreshes viewport metrics from the current frame and re-clamps the
    /// offset so it can never point past the top of the content.
    pub fn sync(&mut self, total_rows: usize, viewport_rows: usize) {
        self.viewport_rows = viewport_rows;
        self.max_offset = total_rows.saturating_sub(viewport_rows);
        self.offset = self.offset.min(self.max_offset);
    }
}

/// Renders `lines` inside `block` as a bottom-pinned scrollable log.
///
/// Wrapped row counts come from `Paragraph::line_count`, which uses the same
/// word-wrapping pass as rendering, so offsets are exact even when lines wrap.
pub fn render_scrolled_lines(
    frame: &mut Frame,
    lines: Vec<Line<'_>>,
    state: &mut ScrollState,
    block: Block,
    area: Rect,
    trim: bool,
) {
    let inner = block.inner(area);
    let paragraph = Paragraph::new(Text::from(lines)).wrap(Wrap { trim });
    let total_rows = paragraph.line_count(inner.width);
    state.sync(total_rows, inner.height as usize);

    let scroll_from_top = state
        .max_offset
        .saturating_sub(state.offset)
        .min(u16::MAX as usize) as u16;
    let log = paragraph.block(block).scroll((scroll_from_top, 0));
    frame.render_widget(log, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synced_state(total_rows: usize, viewport_rows: usize) -> ScrollState {
        let mut state = ScrollState::default();
        state.sync(total_rows, viewport_rows);
        state
    }

    #[test]
    fn new_state_is_pinned() {
        let state = ScrollState::default();
        assert!(state.is_pinned());
        assert_eq!(state.offset(), 0);
    }

    #[test]
    fn scroll_up_clamps_to_max_offset() {
        let mut state = synced_state(20, 5);
        state.scroll_up(100);
        assert_eq!(state.offset(), 15);
    }

    #[test]
    fn scroll_up_before_sync_is_noop() {
        let mut state = ScrollState::default();
        state.scroll_up(10);
        assert_eq!(state.offset(), 0);
    }

    #[test]
    fn scroll_down_saturates_at_zero() {
        let mut state = synced_state(20, 5);
        state.scroll_up(3);
        state.scroll_down(100);
        assert!(state.is_pinned());
    }

    #[test]
    fn page_scroll_uses_viewport_rows() {
        let mut state = synced_state(30, 10);
        state.page_up();
        assert_eq!(state.offset(), 10);
        state.page_down();
        assert_eq!(state.offset(), 0);
    }

    #[test]
    fn page_up_before_sync_scrolls_at_least_one_row() {
        let mut state = ScrollState::default();
        state.sync(5, 0);
        state.page_up();
        assert_eq!(state.offset(), 1);
    }

    #[test]
    fn sync_reclamps_when_content_shrinks() {
        let mut state = synced_state(20, 5);
        state.scroll_up(15);
        state.sync(8, 5);
        assert_eq!(state.offset(), 3);
    }

    #[test]
    fn pin_to_bottom_resets_offset() {
        let mut state = synced_state(20, 5);
        state.scroll_up(7);
        state.pin_to_bottom();
        assert!(state.is_pinned());
    }

    #[test]
    fn line_count_counts_wrapped_rows() {
        let paragraph = Paragraph::new("hello wide world").wrap(Wrap { trim: true });
        assert_eq!(paragraph.line_count(20), 1);
        assert_eq!(paragraph.line_count(6), 3);
    }

    #[test]
    fn line_count_at_zero_width_is_zero() {
        let paragraph = Paragraph::new("hello").wrap(Wrap { trim: true });
        assert_eq!(paragraph.line_count(0), 0);
    }
}
