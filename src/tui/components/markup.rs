use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::theme::{Markup, MessageTheme, StyleKey, StyleOverrides};

/// Parses markdown-ish description/narration text into themed `Line`s.
///
/// Only inline styling is honored (bold, emphasis, inline code as highlight).
/// Unsupported constructs (headings, lists, blockquotes, links, images, html)
/// render their inner text as plain spans in the surrounding style; they never
/// cause a panic.
pub fn parse(text: &str, theme: &MessageTheme, overrides: &StyleOverrides) -> Vec<Line<'static>> {
    let mut builder = LineBuilder::new(theme, overrides);
    for event in Parser::new(text) {
        builder.handle_event(event);
    }
    builder.finish()
}

struct LineBuilder<'a> {
    theme: &'a MessageTheme,
    overrides: &'a StyleOverrides,
    style_stack: Vec<Style>,
    current_spans: Vec<Span<'static>>,
    lines: Vec<Line<'static>>,
}

impl<'a> LineBuilder<'a> {
    fn new(theme: &'a MessageTheme, overrides: &'a StyleOverrides) -> Self {
        Self {
            theme,
            overrides,
            style_stack: vec![Style::default()],
            current_spans: Vec::new(),
            lines: Vec::new(),
        }
    }

    fn current_style(&self) -> Style {
        *self.style_stack.last().unwrap_or(&Style::default())
    }

    fn resolve(&self, markup: Markup) -> Style {
        self.theme
            .resolve(StyleKey::Markup(markup), Some(self.overrides))
    }

    fn push_style(&mut self, markup: Markup) {
        let patched = self.current_style().patch(self.resolve(markup));
        self.style_stack.push(patched);
    }

    fn pop_style(&mut self) {
        if self.style_stack.len() > 1 {
            self.style_stack.pop();
        }
    }

    fn push_text(&mut self, text: String) {
        self.current_spans
            .push(Span::styled(text, self.current_style()));
    }

    fn flush_line(&mut self) {
        let spans = std::mem::take(&mut self.current_spans);
        self.lines.push(Line::from(spans));
    }

    fn flush_line_if_nonempty(&mut self) {
        if !self.current_spans.is_empty() {
            self.flush_line();
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.handle_start(tag),
            Event::End(tag_end) => self.handle_end(tag_end),
            Event::Text(text) => self.push_text(text.into_string()),
            Event::Code(text) => {
                let style = self.current_style().patch(self.resolve(Markup::Highlight));
                self.current_spans
                    .push(Span::styled(text.into_string(), style));
            }
            Event::SoftBreak | Event::HardBreak => self.flush_line(),
            _ => {}
        }
    }

    fn handle_start(&mut self, tag: Tag) {
        match tag {
            Tag::Strong => self.push_style(Markup::Bold),
            Tag::Emphasis => self.push_style(Markup::Emphasis),
            _ => {}
        }
    }

    fn handle_end(&mut self, tag_end: TagEnd) {
        match tag_end {
            TagEnd::Strong | TagEnd::Emphasis => self.pop_style(),
            TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::Item
            | TagEnd::BlockQuote(_)
            | TagEnd::CodeBlock => self.flush_line_if_nonempty(),
            _ => {}
        }
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        self.flush_line_if_nonempty();
        self.lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Modifier};

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
    fn plain_text_passthrough() {
        let theme = MessageTheme;
        let overrides = StyleOverrides::new();
        let lines = parse("Hello, traveller.", &theme, &overrides);
        assert_eq!(lines.len(), 1);
        assert_eq!(plain_text(&lines), "Hello, traveller.");
        assert_eq!(lines[0].spans[0].style, Style::default());
    }

    #[test]
    fn bold_applies_theme_bold_style() {
        let theme = MessageTheme;
        let overrides = StyleOverrides::new();
        let lines = parse("**bold**", &theme, &overrides);
        assert_eq!(plain_text(&lines), "bold");
        assert_eq!(
            lines[0].spans[0].style,
            Style::default().add_modifier(Modifier::BOLD)
        );
    }

    #[test]
    fn emphasis_applies_theme_emphasis_style() {
        let theme = MessageTheme;
        let overrides = StyleOverrides::new();
        let lines = parse("*em*", &theme, &overrides);
        assert_eq!(plain_text(&lines), "em");
        assert_eq!(
            lines[0].spans[0].style,
            Style::default().add_modifier(Modifier::ITALIC)
        );
    }

    #[test]
    fn nested_bold_and_emphasis_compose() {
        let theme = MessageTheme;
        let overrides = StyleOverrides::new();
        let lines = parse("**bold *and em***", &theme, &overrides);
        assert_eq!(plain_text(&lines), "bold and em");
        let bold_span = &lines[0].spans[0];
        assert_eq!(bold_span.content.as_ref(), "bold ");
        assert_eq!(
            bold_span.style,
            Style::default().add_modifier(Modifier::BOLD)
        );
        let nested_span = &lines[0].spans[1];
        assert_eq!(nested_span.content.as_ref(), "and em");
        assert_eq!(
            nested_span.style,
            Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC)
        );
    }

    #[test]
    fn inline_code_applies_highlight_style() {
        let theme = MessageTheme;
        let overrides = StyleOverrides::new();
        let lines = parse("`code`", &theme, &overrides);
        assert_eq!(plain_text(&lines), "code");
        assert_eq!(
            lines[0].spans[0].style,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        );
    }

    #[test]
    fn soft_break_starts_new_line() {
        let theme = MessageTheme;
        let overrides = StyleOverrides::new();
        let lines = parse("line one\nline two", &theme, &overrides);
        assert_eq!(lines.len(), 2);
        assert_eq!(plain_text(&lines), "line one\nline two");
    }

    #[test]
    fn hard_break_starts_new_line() {
        let theme = MessageTheme;
        let overrides = StyleOverrides::new();
        let lines = parse("line one  \nline two", &theme, &overrides);
        assert_eq!(lines.len(), 2);
        assert_eq!(plain_text(&lines), "line one\nline two");
    }

    #[test]
    fn unsupported_heading_renders_as_plain_text() {
        let theme = MessageTheme;
        let overrides = StyleOverrides::new();
        let lines = parse("# A Heading", &theme, &overrides);
        assert_eq!(plain_text(&lines), "A Heading");
    }

    #[test]
    fn unsupported_list_renders_as_plain_text() {
        let theme = MessageTheme;
        let overrides = StyleOverrides::new();
        let lines = parse("- one\n- two", &theme, &overrides);
        assert_eq!(plain_text(&lines), "one\ntwo");
    }

    #[test]
    fn unclosed_marker_does_not_panic() {
        let theme = MessageTheme;
        let overrides = StyleOverrides::new();
        let lines = parse("**bold with no close", &theme, &overrides);
        assert_eq!(plain_text(&lines), "**bold with no close");
    }

    #[test]
    fn override_takes_precedence_over_default() {
        let theme = MessageTheme;
        let mut overrides = StyleOverrides::new();
        overrides.insert(
            StyleKey::Markup(Markup::Bold),
            Style::default().fg(Color::Red),
        );
        let lines = parse("**bold**", &theme, &overrides);
        assert_eq!(lines[0].spans[0].style, Style::default().fg(Color::Red));
    }

    #[test]
    fn empty_text_returns_no_lines() {
        let theme = MessageTheme;
        let overrides = StyleOverrides::new();
        let lines = parse("", &theme, &overrides);
        assert!(lines.is_empty());
    }
}
