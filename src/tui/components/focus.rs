use ratatui::style::{Color, Style};

pub fn focus_style(is_focused: bool) -> Style {
    if is_focused {
        Style::default()
    } else {
        Style::default().fg(Color::DarkGray)
    }
}
