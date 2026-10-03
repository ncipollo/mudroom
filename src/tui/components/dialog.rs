use ratatui::layout::Rect;

/// Centers a `width` x `height` box within `area`, clamping to `area`'s bounds so the overlay
/// never renders larger than the frame it's drawn into. Shared by every popup dialog so they all
/// center the same way.
pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
