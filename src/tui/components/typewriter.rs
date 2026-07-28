use std::time::Duration;

use ratatui::text::{Line, Span};

/// Default typewriter reveal rate, in characters per second.
pub const DEFAULT_REVEAL_RATE: f64 = 80.0;

/// Tracks progressive reveal of a single message in the log.
///
/// `carry` retains the fractional character left over between ticks so slow
/// rates (or fast ticks) don't lose precision to integer truncation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypewriterState {
    pub message_index: usize,
    pub revealed_chars: usize,
    carry: f64,
}

impl TypewriterState {
    pub fn new(message_index: usize) -> Self {
        Self {
            message_index,
            revealed_chars: 0,
            carry: 0.0,
        }
    }

    pub fn advance(&mut self, rate_chars_per_sec: f64, tick: Duration, total_chars: usize) {
        self.carry += rate_chars_per_sec * tick.as_secs_f64();
        let whole = self.carry.floor();
        self.revealed_chars = (self.revealed_chars + whole as usize).min(total_chars);
        self.carry -= whole;
    }

    pub fn is_caught_up(&self, total_chars: usize) -> bool {
        self.revealed_chars >= total_chars
    }
}

/// Total revealable character count across already-parsed spans.
pub fn visible_len(lines: &[Line<'static>]) -> usize {
    lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.chars().count())
                .sum::<usize>()
        })
        .sum()
}

/// Truncates parsed lines to the first `revealed_chars` characters,
/// preserving span styling and dropping unrevealed lines/spans entirely.
pub fn truncate_lines(lines: &[Line<'static>], revealed_chars: usize) -> Vec<Line<'static>> {
    let mut remaining = revealed_chars;
    let mut out = Vec::new();

    for line in lines {
        if remaining == 0 {
            break;
        }

        let mut spans = Vec::new();
        for span in &line.spans {
            if remaining == 0 {
                break;
            }
            let span_len = span.content.chars().count();
            if remaining >= span_len {
                spans.push(span.clone());
                remaining -= span_len;
            } else {
                let truncated: String = span.content.chars().take(remaining).collect();
                spans.push(Span::styled(truncated, span.style));
                remaining = 0;
            }
        }
        out.push(Line::from(spans));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Style};

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
    fn advance_accumulates_fractional_chars_across_ticks() {
        let mut state = TypewriterState::new(0);
        // 40 chars/sec at 50ms ticks = 2 chars/tick exactly.
        state.advance(40.0, Duration::from_millis(50), 100);
        assert_eq!(state.revealed_chars, 2);
        state.advance(40.0, Duration::from_millis(50), 100);
        assert_eq!(state.revealed_chars, 4);
    }

    #[test]
    fn advance_carries_fractional_remainder() {
        let mut state = TypewriterState::new(0);
        // 10 chars/sec at 50ms ticks = 0.5 chars/tick; carries across ticks.
        state.advance(10.0, Duration::from_millis(50), 100);
        assert_eq!(state.revealed_chars, 0);
        state.advance(10.0, Duration::from_millis(50), 100);
        assert_eq!(state.revealed_chars, 1);
    }

    #[test]
    fn advance_clamps_at_total_chars() {
        let mut state = TypewriterState::new(0);
        state.advance(1000.0, Duration::from_secs(1), 5);
        assert_eq!(state.revealed_chars, 5);
    }

    #[test]
    fn is_caught_up_reflects_total() {
        let mut state = TypewriterState::new(0);
        assert!(!state.is_caught_up(5));
        state.advance(1000.0, Duration::from_secs(1), 5);
        assert!(state.is_caught_up(5));
    }

    #[test]
    fn visible_len_sums_span_chars() {
        let lines = vec![
            Line::from(vec![Span::raw("abc"), Span::raw("de")]),
            Line::from(vec![Span::raw("f")]),
        ];
        assert_eq!(visible_len(&lines), 6);
    }

    #[test]
    fn truncate_lines_mid_span() {
        let lines = vec![Line::from(vec![Span::raw("hello world")])];
        let truncated = truncate_lines(&lines, 5);
        assert_eq!(plain_text(&truncated), "hello");
    }

    #[test]
    fn truncate_lines_preserves_style() {
        let style = Style::default().fg(Color::Yellow);
        let lines = vec![Line::from(vec![Span::styled("hello", style)])];
        let truncated = truncate_lines(&lines, 3);
        assert_eq!(truncated[0].spans[0].style, style);
        assert_eq!(truncated[0].spans[0].content.as_ref(), "hel");
    }

    #[test]
    fn truncate_lines_at_span_boundary() {
        let lines = vec![Line::from(vec![Span::raw("abc"), Span::raw("def")])];
        let truncated = truncate_lines(&lines, 3);
        assert_eq!(plain_text(&truncated), "abc");
        assert_eq!(truncated[0].spans.len(), 1);
    }

    #[test]
    fn truncate_lines_drops_unrevealed_lines() {
        let lines = vec![
            Line::from(vec![Span::raw("abc")]),
            Line::from(vec![Span::raw("def")]),
        ];
        let truncated = truncate_lines(&lines, 3);
        assert_eq!(plain_text(&truncated), "abc");
        assert_eq!(truncated.len(), 1);
    }

    #[test]
    fn truncate_lines_zero_chars_returns_empty() {
        let lines = vec![Line::from(vec![Span::raw("abc")])];
        let truncated = truncate_lines(&lines, 0);
        assert!(truncated.is_empty());
    }

    #[test]
    fn truncate_lines_full_length_matches_original() {
        let lines = vec![
            Line::from(vec![Span::raw("abc")]),
            Line::from(vec![Span::raw("def")]),
        ];
        let total = visible_len(&lines);
        let truncated = truncate_lines(&lines, total);
        assert_eq!(plain_text(&truncated), "abc\ndef");
    }
}
