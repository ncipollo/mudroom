use ratatui::text::{Line, Span};

use crate::tui::components::markup;
use crate::tui::components::theme::{MessageKind, MessageTheme, StyleKey, StyleOverrides};

#[derive(Debug, Clone)]
pub struct AppMessage {
    pub text: String,
    pub kind: MessageKind,
    pub lines: Vec<Line<'static>>,
}

impl AppMessage {
    pub fn normal(text: impl Into<String>, theme: &MessageTheme) -> Self {
        Self::with_kind(text, MessageKind::Narration, theme)
    }

    pub fn command(text: impl Into<String>, theme: &MessageTheme) -> Self {
        Self::with_kind(text, MessageKind::PlayerCommand, theme)
    }

    pub fn system(text: impl Into<String>, theme: &MessageTheme) -> Self {
        Self::with_kind(text, MessageKind::System, theme)
    }

    pub fn debug(text: impl Into<String>, theme: &MessageTheme) -> Self {
        Self::with_kind(text, MessageKind::Debug, theme)
    }

    fn with_kind(text: impl Into<String>, kind: MessageKind, theme: &MessageTheme) -> Self {
        let text = text.into();
        let lines = styled_lines(&text, kind, theme);
        Self { text, kind, lines }
    }

    /// Appends `chunk` to the message text and re-parses the accumulated
    /// text, so a streamed SSE message stays correctly styled as it grows.
    pub fn append(&mut self, chunk: &str, theme: &MessageTheme) {
        self.text.push_str(chunk);
        self.lines = styled_lines(&self.text, self.kind, theme);
    }
}

fn styled_lines(text: &str, kind: MessageKind, theme: &MessageTheme) -> Vec<Line<'static>> {
    let overrides = StyleOverrides::new();
    let base_style = theme.resolve(StyleKey::Message(kind), None);

    let lines = markup::parse(text, theme, &overrides)
        .into_iter()
        .map(|line| {
            let spans: Vec<Span<'static>> = line
                .spans
                .into_iter()
                .map(|span| Span::styled(span.content, base_style.patch(span.style)))
                .collect();
            Line::from(spans)
        });

    if kind == MessageKind::PlayerCommand {
        lines.map(prefix_command_line).collect()
    } else {
        lines.collect()
    }
}

fn prefix_command_line(mut line: Line<'static>) -> Line<'static> {
    line.spans.insert(0, Span::raw("> "));
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Modifier;

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
    fn normal_message_parses_markdown() {
        let theme = MessageTheme;
        let msg = AppMessage::normal("**bold**", &theme);
        assert_eq!(msg.text, "**bold**");
        assert_eq!(plain_text(&msg.lines), "bold");
        assert!(
            msg.lines[0].spans[0]
                .style
                .add_modifier
                .contains(Modifier::BOLD)
        );
    }

    #[test]
    fn command_message_prefixes_every_line() {
        let theme = MessageTheme;
        let msg = AppMessage::command("line one\nline two", &theme);
        assert_eq!(msg.lines.len(), 2);
        assert_eq!(plain_text(&msg.lines), "> line one\n> line two");
    }

    #[test]
    fn system_and_debug_use_expected_kind() {
        let theme = MessageTheme;
        assert_eq!(AppMessage::system("hi", &theme).kind, MessageKind::System);
        assert_eq!(AppMessage::debug("hi", &theme).kind, MessageKind::Debug);
    }

    #[test]
    fn append_accumulates_and_reparses() {
        let theme = MessageTheme;
        let mut msg = AppMessage::normal("**bo", &theme);
        msg.append("ld**", &theme);
        assert_eq!(msg.text, "**bold**");
        assert_eq!(plain_text(&msg.lines), "bold");
        assert!(
            msg.lines[0].spans[0]
                .style
                .add_modifier
                .contains(Modifier::BOLD)
        );
    }
}
