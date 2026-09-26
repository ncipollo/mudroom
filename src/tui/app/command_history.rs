/// Tracks locally-submitted commands for Up/Down recall in free-text input prompts.
/// In-memory only, never persisted or synced — resets on client restart.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CommandHistory {
    entries: Vec<String>,
    cursor: Option<usize>,
    draft: String,
}

impl CommandHistory {
    pub fn push(&mut self, command: String) {
        self.cursor = None;
        if command.is_empty() {
            return;
        }
        if self.entries.last().is_some_and(|last| last == &command) {
            return;
        }
        self.entries.push(command);
    }

    pub fn prev(&mut self, current_input: &str) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }
        let index = match self.cursor {
            None => {
                self.draft = current_input.to_string();
                self.entries.len() - 1
            }
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.cursor = Some(index);
        self.entries.get(index).cloned()
    }

    pub fn next(&mut self) -> Option<String> {
        let index = self.cursor?;
        if index + 1 < self.entries.len() {
            self.cursor = Some(index + 1);
            self.entries.get(index + 1).cloned()
        } else {
            self.cursor = None;
            Some(std::mem::take(&mut self.draft))
        }
    }

    pub fn reset_navigation(&mut self) {
        self.cursor = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_ignores_empty_input() {
        let mut history = CommandHistory::default();
        history.push(String::new());
        assert_eq!(history.prev(""), None);
    }

    #[test]
    fn push_ignores_immediate_consecutive_duplicate() {
        let mut history = CommandHistory::default();
        history.push("look".to_string());
        history.push("look".to_string());
        assert_eq!(history.prev(""), Some("look".to_string()));
        assert_eq!(history.prev("look"), Some("look".to_string()));
    }

    #[test]
    fn push_allows_non_consecutive_duplicate() {
        let mut history = CommandHistory::default();
        history.push("look".to_string());
        history.push("north".to_string());
        history.push("look".to_string());
        assert_eq!(history.prev(""), Some("look".to_string()));
        assert_eq!(history.prev("look"), Some("north".to_string()));
        assert_eq!(history.prev("north"), Some("look".to_string()));
    }

    #[test]
    fn prev_with_empty_history_returns_none() {
        let mut history = CommandHistory::default();
        assert_eq!(history.prev("typing"), None);
    }

    #[test]
    fn prev_walks_backward_and_clamps_at_oldest() {
        let mut history = CommandHistory::default();
        history.push("a".to_string());
        history.push("b".to_string());
        history.push("c".to_string());
        assert_eq!(history.prev(""), Some("c".to_string()));
        assert_eq!(history.prev("c"), Some("b".to_string()));
        assert_eq!(history.prev("b"), Some("a".to_string()));
        assert_eq!(history.prev("a"), Some("a".to_string()));
    }

    #[test]
    fn next_without_navigation_returns_none() {
        let mut history = CommandHistory::default();
        history.push("a".to_string());
        assert_eq!(history.next(), None);
    }

    #[test]
    fn next_restores_draft_past_newest() {
        let mut history = CommandHistory::default();
        history.push("a".to_string());
        history.push("b".to_string());
        history.prev("draft text");
        assert_eq!(history.next(), Some("draft text".to_string()));
        assert_eq!(history.next(), None);
    }

    #[test]
    fn next_walks_forward_through_entries() {
        let mut history = CommandHistory::default();
        history.push("a".to_string());
        history.push("b".to_string());
        history.push("c".to_string());
        history.prev("");
        history.prev("c");
        history.prev("b");
        assert_eq!(history.next(), Some("b".to_string()));
        assert_eq!(history.next(), Some("c".to_string()));
        assert_eq!(history.next(), Some(String::new()));
    }

    #[test]
    fn reset_navigation_makes_next_prev_snapshot_fresh_draft() {
        let mut history = CommandHistory::default();
        history.push("a".to_string());
        history.push("b".to_string());
        history.prev("");
        history.reset_navigation();
        assert_eq!(history.next(), None);
        assert_eq!(history.prev("edited"), Some("b".to_string()));
        assert_eq!(history.next(), Some("edited".to_string()));
    }

    #[test]
    fn push_after_navigating_resets_cursor() {
        let mut history = CommandHistory::default();
        history.push("a".to_string());
        history.prev("");
        history.push("b".to_string());
        assert_eq!(history.next(), None);
    }
}
