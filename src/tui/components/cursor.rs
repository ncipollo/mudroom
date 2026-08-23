use ratatui::{Frame, layout::Rect, text::Line};

/// Places the terminal cursor at the end of `text` inside `inner`, offset by `prefix_cols` (the
/// width of a leading prompt like `"> "`). Without this, key handlers update `app.input` and the
/// widget repaints it, but the terminal's own cursor never moves — nothing on screen shows where
/// typed characters are landing. Clamped to stay within `inner`'s width so a line that overflows
/// the box doesn't push the cursor outside it.
pub fn place_at_end(frame: &mut Frame, inner: Rect, prefix_cols: u16, text: &str) {
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let text_width = Line::from(text).width() as u16;
    let max_x = inner.x + inner.width - 1;
    let x = (inner.x + prefix_cols + text_width).min(max_x);
    frame.set_cursor_position((x, inner.y));
}

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::widgets::Block;
    use ratatui::{Terminal, layout::Rect};

    use super::*;

    fn cursor_position(
        width: u16,
        height: u16,
        inner: Rect,
        prefix_cols: u16,
        text: &str,
    ) -> (u16, u16) {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| place_at_end(frame, inner, prefix_cols, text))
            .unwrap();
        terminal.get_cursor_position().unwrap().into()
    }

    #[test]
    fn places_cursor_after_the_prefix_and_text() {
        let inner = Block::default().inner(Rect::new(0, 0, 20, 3));
        let (x, y) = cursor_position(20, 3, inner, 2, "hi");
        assert_eq!((x, y), (inner.x + 4, inner.y));
    }

    #[test]
    fn clamps_to_the_inner_width_when_text_overflows() {
        let inner = Rect::new(0, 0, 5, 1);
        let (x, _) = cursor_position(5, 1, inner, 2, "way too long to fit");
        assert_eq!(x, inner.x + inner.width - 1);
    }

    #[test]
    fn zero_size_area_is_a_noop() {
        let backend = TestBackend::new(5, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let result = terminal.draw(|frame| place_at_end(frame, Rect::new(0, 0, 0, 0), 2, "hi"));
        assert!(result.is_ok());
    }
}
