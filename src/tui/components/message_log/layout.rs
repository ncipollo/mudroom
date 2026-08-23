use ratatui::text::{Line, Text};
use ratatui::widgets::{Paragraph, Wrap};

use crate::tui::app::AppMessage;

/// Caches each message's wrapped row count at the last-rendered width, so the log's total row
/// count doesn't require re-wrapping the entire history every frame — only genuinely new or
/// changed messages (or a width change) trigger a rewrap.
#[derive(Debug, Clone, Default)]
pub struct LogLayout {
    width: u16,
    row_counts: Vec<usize>,
    total_rows: usize,
    streaming: Option<(usize, usize)>,
}

impl LogLayout {
    pub fn total_rows(&self) -> usize {
        self.total_rows
    }

    pub fn row_count(&self, index: usize) -> usize {
        self.row_counts.get(index).copied().unwrap_or(0)
    }

    /// Brings the cache up to date with `messages` at `width`, doing only as much work as the
    /// change requires: nothing on an unchanged log (e.g. a keystroke), one wrap for a newly
    /// pushed message or for `streaming_index` growing in place, or a full rebuild on resize or
    /// a shrunk log (e.g. `AgentConversation` clearing history).
    pub fn sync(&mut self, messages: &[AppMessage], streaming_index: Option<usize>, width: u16) {
        if self.width != width || messages.len() < self.row_counts.len() {
            self.rebuild(messages, streaming_index, width);
            return;
        }
        for msg in &messages[self.row_counts.len()..] {
            self.push(msg, width);
        }
        self.sync_streaming(messages, streaming_index, width);
    }

    fn rebuild(&mut self, messages: &[AppMessage], streaming_index: Option<usize>, width: u16) {
        self.width = width;
        self.row_counts.clear();
        self.total_rows = 0;
        self.streaming = None;
        for msg in messages {
            self.push(msg, width);
        }
        self.sync_streaming(messages, streaming_index, width);
    }

    fn push(&mut self, msg: &AppMessage, width: u16) {
        let rows = wrapped_row_count(&msg.lines, width);
        self.row_counts.push(rows);
        self.total_rows += rows;
    }

    /// Re-wraps the in-flight streamed message when its text has grown since the last sync.
    /// Keyed by `streaming_index` (rather than assuming the last message) so an interleaved
    /// push — e.g. a debug ping logged mid-stream — can't leave the streaming message's cached
    /// row count stale.
    fn sync_streaming(
        &mut self,
        messages: &[AppMessage],
        streaming_index: Option<usize>,
        width: u16,
    ) {
        let Some(index) = streaming_index else {
            self.streaming = None;
            return;
        };
        let Some(msg) = messages.get(index) else {
            return;
        };
        let text_len = msg.text.len();
        if self.streaming == Some((index, text_len)) {
            return;
        }
        let rows = wrapped_row_count(&msg.lines, width);
        if let Some(slot) = self.row_counts.get_mut(index) {
            self.total_rows = self.total_rows - *slot + rows;
            *slot = rows;
        }
        self.streaming = Some((index, text_len));
    }
}

/// Wrapped row count for a single message's lines, using the same wrapping pass
/// (`Wrap { trim: true }`) the log is painted with.
pub fn wrapped_row_count(lines: &[Line<'static>], width: u16) -> usize {
    Paragraph::new(Text::from(lines.to_vec()))
        .wrap(Wrap { trim: true })
        .line_count(width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::components::theme::MessageTheme;

    fn messages(texts: &[&str]) -> Vec<AppMessage> {
        let theme = MessageTheme;
        texts
            .iter()
            .map(|t| AppMessage::normal(*t, &theme))
            .collect()
    }

    #[test]
    fn sync_computes_row_counts_for_new_messages() {
        let mut layout = LogLayout::default();
        let msgs = messages(&["hello", "world"]);
        layout.sync(&msgs, None, 80);
        assert_eq!(layout.row_count(0), 1);
        assert_eq!(layout.row_count(1), 1);
        assert_eq!(layout.total_rows(), 2);
    }

    #[test]
    fn sync_only_wraps_newly_appended_messages() {
        let mut layout = LogLayout::default();
        let mut msgs = messages(&["hello"]);
        layout.sync(&msgs, None, 80);
        msgs.push(AppMessage::normal("world", &MessageTheme));
        layout.sync(&msgs, None, 80);
        assert_eq!(layout.row_count(1), 1);
        assert_eq!(layout.total_rows(), 2);
    }

    #[test]
    fn sync_is_a_noop_when_nothing_changed() {
        let mut layout = LogLayout::default();
        let msgs = messages(&["hello wide world"]);
        layout.sync(&msgs, None, 6);
        let before = layout.clone();
        layout.sync(&msgs, None, 6);
        assert_eq!(layout.row_counts, before.row_counts);
        assert_eq!(layout.total_rows(), before.total_rows());
    }

    #[test]
    fn width_change_triggers_full_rebuild() {
        let mut layout = LogLayout::default();
        let msgs = messages(&["hello wide world"]);
        layout.sync(&msgs, None, 20);
        assert_eq!(layout.total_rows(), 1);
        layout.sync(&msgs, None, 6);
        assert_eq!(layout.total_rows(), 3);
    }

    #[test]
    fn shrinking_log_triggers_rebuild() {
        let mut layout = LogLayout::default();
        let msgs = messages(&["one", "two", "three"]);
        layout.sync(&msgs, None, 80);
        let shrunk = messages(&["only"]);
        layout.sync(&shrunk, None, 80);
        assert_eq!(layout.total_rows(), 1);
        assert_eq!(layout.row_count(1), 0);
    }

    #[test]
    fn streaming_growth_updates_only_the_streaming_entry() {
        let mut layout = LogLayout::default();
        let mut msgs = messages(&["intro"]);
        msgs.push(AppMessage::normal("hi", &MessageTheme));
        layout.sync(&msgs, Some(1), 80);
        assert_eq!(layout.row_count(0), 1);
        assert_eq!(layout.row_count(1), 1);

        if let Some(msg) = msgs.get_mut(1) {
            msg.append(" there friend", &MessageTheme);
        }
        layout.sync(&msgs, Some(1), 80);
        assert_eq!(layout.row_count(0), 1);
        assert_eq!(layout.row_count(1), 1);
        assert_eq!(layout.total_rows(), 2);
    }

    #[test]
    fn streaming_index_survives_interleaved_pushes() {
        let mut layout = LogLayout::default();
        let mut msgs = messages(&["room description"]);
        msgs.push(AppMessage::normal("streaming reply", &MessageTheme));
        layout.sync(&msgs, Some(1), 80);

        // A debug ping lands mid-stream, pushed after the streaming message.
        msgs.push(AppMessage::debug("[ping received]", &MessageTheme));
        layout.sync(&msgs, Some(1), 80);

        if let Some(msg) = msgs.get_mut(1) {
            msg.append(" with more content", &MessageTheme);
        }
        layout.sync(&msgs, Some(1), 80);

        assert_eq!(layout.row_count(1), wrapped_row_count(&msgs[1].lines, 80));
    }
}
