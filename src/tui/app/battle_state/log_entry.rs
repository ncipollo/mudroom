use ratatui::text::{Line, Span};

use crate::game::engagement::battle::BattleMessage;
use crate::tui::components::theme::{BattleKind, MessageTheme, StyleKey};

#[derive(Debug, Clone)]
pub struct BattleLogEntry {
    pub message: BattleMessage,
    pub lines: Vec<Line<'static>>,
}

/// Only these battle updates type out with the reveal ticker; everything
/// else (pending-attack declarations, effect text, meta lines, expirations)
/// is low-signal and appears instantly.
pub fn is_major(message: &BattleMessage) -> bool {
    matches!(
        message,
        BattleMessage::AbilityCast { .. }
            | BattleMessage::EntityDied { .. }
            | BattleMessage::PhaseChange { .. }
    )
}

pub fn to_entry(message: BattleMessage, theme: &MessageTheme) -> BattleLogEntry {
    let lines = styled_lines(&message, theme);
    BattleLogEntry { message, lines }
}

fn styled_lines(message: &BattleMessage, theme: &MessageTheme) -> Vec<Line<'static>> {
    if let BattleMessage::EffectText(text) = message {
        let style = theme.resolve(StyleKey::Battle(BattleKind::EffectText), None);
        return vec![Line::from(vec![
            Span::raw("  "),
            Span::styled(text.clone(), style),
        ])];
    }
    let style = theme.resolve(StyleKey::Battle(battle_kind(message)), None);
    vec![Line::from(Span::styled(message.to_string(), style))]
}

fn battle_kind(message: &BattleMessage) -> BattleKind {
    match message {
        BattleMessage::PhaseChange { .. } => BattleKind::PhaseChange,
        BattleMessage::AbilityCast { .. } => BattleKind::AbilityCast,
        BattleMessage::EntityDied { .. } => BattleKind::EntityDied,
        BattleMessage::PendingAttack { .. } => BattleKind::PendingAttack,
        BattleMessage::Meta(_) => BattleKind::Meta,
        BattleMessage::EffectText(_) => BattleKind::EffectText,
        BattleMessage::EffectExpired { .. } => BattleKind::EffectExpired,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Modifier, Style};

    fn plain_text(lines: &[Line<'static>]) -> String {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn is_major_true_for_ability_cast_death_and_phase_change() {
        assert!(is_major(&BattleMessage::AbilityCast {
            caster_name: "Goblin".into(),
            target_name: "Player".into(),
            ability_name: "Slash".into(),
        }));
        assert!(is_major(&BattleMessage::EntityDied {
            name: "Goblin".into()
        }));
        assert!(is_major(&BattleMessage::PhaseChange {
            phase: crate::game::engagement::battle::BattlePhase::ResolveAbilities
        }));
    }

    #[test]
    fn is_major_false_for_low_signal_messages() {
        assert!(!is_major(&BattleMessage::PendingAttack {
            caster_name: "Goblin".into(),
            ability_name: "Slash".into(),
            target_name: "Player".into(),
            target_id: 1,
        }));
        assert!(!is_major(&BattleMessage::Meta("hi".into())));
        assert!(!is_major(&BattleMessage::EffectText("burns".into())));
        assert!(!is_major(&BattleMessage::EffectExpired {
            entity_name: "Goblin".into(),
            effect_name: "Poison".into(),
        }));
    }

    #[test]
    fn to_entry_styles_entity_died_red_bold() {
        let theme = MessageTheme;
        let entry = to_entry(
            BattleMessage::EntityDied {
                name: "Goblin".into(),
            },
            &theme,
        );
        assert_eq!(plain_text(&entry.lines), "Goblin has been defeated!");
        assert_eq!(
            entry.lines[0].spans[0].style,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        );
    }

    #[test]
    fn to_entry_indents_effect_text() {
        let theme = MessageTheme;
        let entry = to_entry(BattleMessage::EffectText("deals 5 damage".into()), &theme);
        assert_eq!(entry.lines[0].spans.len(), 2);
        assert_eq!(entry.lines[0].spans[0].content.as_ref(), "  ");
        assert_eq!(plain_text(&entry.lines), "  deals 5 damage");
    }
}
