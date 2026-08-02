use std::collections::HashMap;

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageKind {
    PlayerCommand,
    Narration,
    System,
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Markup {
    Bold,
    Emphasis,
    Highlight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BattleKind {
    PhaseChange,
    AbilityCast,
    EntityDied,
    PendingAttack,
    Meta,
    EffectText,
    EffectExpired,
    TargetedDivider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleKey {
    Message(MessageKind),
    Markup(Markup),
    Battle(BattleKind),
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
        StyleKey::Battle(kind) => default_battle_style(kind),
    }
}

fn default_message_style(kind: MessageKind) -> Style {
    match kind {
        MessageKind::PlayerCommand => Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
        MessageKind::Narration => Style::default(),
        MessageKind::System => Style::default().fg(Color::Green),
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

fn default_battle_style(kind: BattleKind) -> Style {
    match kind {
        BattleKind::PhaseChange => Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC | Modifier::DIM),
        BattleKind::AbilityCast => Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
        BattleKind::EntityDied => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        BattleKind::PendingAttack => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::ITALIC),
        BattleKind::Meta => Style::default().fg(Color::White),
        BattleKind::EffectText => Style::default().fg(Color::Gray),
        BattleKind::EffectExpired => Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
        BattleKind::TargetedDivider => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
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

    #[test]
    fn resolve_returns_default_for_battle_kind() {
        let theme = MessageTheme;
        let style = theme.resolve(StyleKey::Battle(BattleKind::EntityDied), None);
        assert_eq!(
            style,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        );
    }

    #[test]
    fn resolve_battle_kind_prefers_override() {
        let theme = MessageTheme;
        let mut overrides = StyleOverrides::new();
        overrides.insert(
            StyleKey::Battle(BattleKind::AbilityCast),
            Style::default().fg(Color::Magenta),
        );
        let style = theme.resolve(StyleKey::Battle(BattleKind::AbilityCast), Some(&overrides));
        assert_eq!(style, Style::default().fg(Color::Magenta));
    }
}
