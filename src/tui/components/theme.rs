use std::collections::HashMap;

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageKind {
    PlayerCommand,
    Narration,
    System,
    /// Placeholder default for a single migrated battle message. Battle
    /// currently has several distinct message variants (see
    /// `battle_message_to_line` in `tui/screens/battle/render.rs`) that a
    /// single coarse kind can't represent — that future migration should
    /// lean on caller-supplied overrides rather than this default alone.
    BattleEvent,
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Markup {
    Bold,
    Emphasis,
    Highlight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleKey {
    Message(MessageKind),
    Markup(Markup),
}

pub type StyleOverrides = HashMap<StyleKey, Style>;

#[derive(Debug, Clone, Copy, Default)]
pub struct MessageTheme;

impl MessageTheme {
    pub fn resolve(&self, key: StyleKey, overrides: Option<&StyleOverrides>) -> Style {
        if let Some(style) = overrides.and_then(|map| map.get(&key)) {
            return *style;
        }
        default_style(key)
    }
}

fn default_style(key: StyleKey) -> Style {
    match key {
        StyleKey::Message(kind) => default_message_style(kind),
        StyleKey::Markup(markup) => default_markup_style(markup),
    }
}

fn default_message_style(kind: MessageKind) -> Style {
    match kind {
        MessageKind::PlayerCommand => Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
        MessageKind::Narration => Style::default(),
        MessageKind::System => Style::default().fg(Color::Green),
        MessageKind::BattleEvent => Style::default().fg(Color::White),
        MessageKind::Debug => Style::default().fg(Color::DarkGray),
    }
}

fn default_markup_style(markup: Markup) -> Style {
    match markup {
        Markup::Bold => Style::default().add_modifier(Modifier::BOLD),
        Markup::Emphasis => Style::default().add_modifier(Modifier::ITALIC),
        Markup::Highlight => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_returns_default_when_no_overrides() {
        let theme = MessageTheme;
        let style = theme.resolve(StyleKey::Message(MessageKind::Narration), None);
        assert_eq!(style, Style::default());
    }

    #[test]
    fn resolve_debug_matches_dark_gray_like_before() {
        let theme = MessageTheme;
        let style = theme.resolve(StyleKey::Message(MessageKind::Debug), None);
        assert_eq!(style, Style::default().fg(Color::DarkGray));
    }

    #[test]
    fn resolve_prefers_override_over_default() {
        let theme = MessageTheme;
        let mut overrides = StyleOverrides::new();
        overrides.insert(
            StyleKey::Message(MessageKind::Narration),
            Style::default().fg(Color::Magenta),
        );
        let style = theme.resolve(StyleKey::Message(MessageKind::Narration), Some(&overrides));
        assert_eq!(style, Style::default().fg(Color::Magenta));
    }

    #[test]
    fn resolve_returns_default_for_markup() {
        let theme = MessageTheme;
        let style = theme.resolve(StyleKey::Markup(Markup::Bold), None);
        assert_eq!(style, Style::default().add_modifier(Modifier::BOLD));
    }
}
