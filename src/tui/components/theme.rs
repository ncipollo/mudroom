use std::collections::HashMap;
use std::str::FromStr;

use ratatui::style::{Color, Modifier, Style};

use crate::network::event::ThemeInfo;

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

fn markup_for_style_key(name: &str) -> Option<Markup> {
    match name {
        "bold" => Some(Markup::Bold),
        "emphasis" => Some(Markup::Emphasis),
        "highlight" => Some(Markup::Highlight),
        _ => {
            tracing::warn!(style_key = %name, "unknown theme style key, ignoring");
            None
        }
    }
}

fn modifier_for_name(name: &str) -> Option<Modifier> {
    match name {
        "bold" => Some(Modifier::BOLD),
        "dim" => Some(Modifier::DIM),
        "italic" => Some(Modifier::ITALIC),
        "underlined" => Some(Modifier::UNDERLINED),
        "slow_blink" => Some(Modifier::SLOW_BLINK),
        "rapid_blink" => Some(Modifier::RAPID_BLINK),
        "reversed" => Some(Modifier::REVERSED),
        "hidden" => Some(Modifier::HIDDEN),
        "crossed_out" => Some(Modifier::CROSSED_OUT),
        _ => {
            tracing::warn!(modifier = %name, "unknown theme modifier, ignoring");
            None
        }
    }
}

fn parse_color(name: &str) -> Option<Color> {
    Color::from_str(name)
        .inspect_err(|_| tracing::warn!(color = %name, "unable to parse theme color, ignoring"))
        .ok()
}

/// Converts a mud-authored theme into the `StyleOverrides` map consumed by `markup::parse`.
/// Unrecognized style keys, colors, or modifiers are warned about and skipped rather than
/// failing the whole theme.
pub fn style_overrides_from_theme(theme: &ThemeInfo) -> StyleOverrides {
    let mut overrides = StyleOverrides::new();
    for (key, style_info) in &theme.styles {
        let Some(markup) = markup_for_style_key(key) else {
            continue;
        };
        let mut style = Style::default();
        if let Some(fg) = style_info.fg.as_deref().and_then(parse_color) {
            style = style.fg(fg);
        }
        if let Some(bg) = style_info.bg.as_deref().and_then(parse_color) {
            style = style.bg(bg);
        }
        for modifier in style_info
            .modifiers
            .iter()
            .filter_map(|m| modifier_for_name(m))
        {
            style = style.add_modifier(modifier);
        }
        overrides.insert(StyleKey::Markup(markup), style);
    }
    overrides
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

    use crate::network::event::ThemeStyleInfo;

    fn theme_info(styles: Vec<(&str, ThemeStyleInfo)>) -> ThemeInfo {
        ThemeInfo {
            id: "eerie".to_string(),
            styles: styles
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }

    #[test]
    fn style_overrides_from_theme_maps_known_keys() {
        let theme = theme_info(vec![(
            "bold",
            ThemeStyleInfo {
                fg: Some("red".to_string()),
                bg: None,
                modifiers: vec!["bold".to_string()],
            },
        )]);
        let overrides = style_overrides_from_theme(&theme);
        let style = overrides.get(&StyleKey::Markup(Markup::Bold)).unwrap();
        assert_eq!(
            *style,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        );
    }

    #[test]
    fn style_overrides_from_theme_ignores_unknown_style_key() {
        let theme = theme_info(vec![(
            "not_a_real_key",
            ThemeStyleInfo {
                fg: Some("red".to_string()),
                bg: None,
                modifiers: vec![],
            },
        )]);
        let overrides = style_overrides_from_theme(&theme);
        assert!(overrides.is_empty());
    }

    #[test]
    fn style_overrides_from_theme_ignores_bad_color_and_modifier() {
        let theme = theme_info(vec![(
            "highlight",
            ThemeStyleInfo {
                fg: Some("not_a_color".to_string()),
                bg: None,
                modifiers: vec!["not_a_modifier".to_string()],
            },
        )]);
        let overrides = style_overrides_from_theme(&theme);
        let style = overrides.get(&StyleKey::Markup(Markup::Highlight)).unwrap();
        assert_eq!(*style, Style::default());
    }

    #[test]
    fn style_overrides_from_theme_applies_bg() {
        let theme = theme_info(vec![(
            "emphasis",
            ThemeStyleInfo {
                fg: None,
                bg: Some("blue".to_string()),
                modifiers: vec![],
            },
        )]);
        let overrides = style_overrides_from_theme(&theme);
        let style = overrides.get(&StyleKey::Markup(Markup::Emphasis)).unwrap();
        assert_eq!(*style, Style::default().bg(Color::Blue));
    }
}
