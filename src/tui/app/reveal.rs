use std::time::Duration;

use super::App;
use crate::tui::components::theme::MessageKind;
use crate::tui::components::typewriter::{self, TypewriterState};

impl App {
    /// Starts revealing the message at `message_index`, if it's narration.
    ///
    /// A burst of narration messages (e.g. a move confirmation followed by a
    /// room description) can be pushed within the same tick, well before the
    /// first one finishes revealing. Rather than letting each new message
    /// steal the reveal from the one in progress, later messages queue up
    /// and reveal in turn.
    pub fn start_reveal(&mut self, message_index: usize) {
        let is_narration = self
            .messages
            .get(message_index)
            .is_some_and(|msg| msg.kind == MessageKind::Narration);
        if !is_narration {
            return;
        }
        if self.reveal.is_some() {
            self.reveal_queue.push_back(message_index);
        } else {
            self.reveal = Some(TypewriterState::new(message_index));
        }
    }

    /// Advances the active reveal by `tick`, moving on to the next queued
    /// reveal once the tracked message is fully shown and no longer
    /// streaming in new chunks.
    pub fn tick_reveal(&mut self, tick: Duration) {
        let Some(index) = self.reveal.as_ref().map(|state| state.message_index) else {
            return;
        };
        let Some(msg) = self.messages.get(index) else {
            self.advance_reveal_queue();
            return;
        };

        let total = typewriter::visible_len(&msg.lines);
        let rate = self.reveal_rate;
        let still_streaming = self.streaming_message_index == Some(index);

        if let Some(state) = self.reveal.as_mut() {
            state.advance(rate, tick, total);
            if state.is_caught_up(total) && !still_streaming {
                self.advance_reveal_queue();
            }
        }
    }

    /// Completes the active reveal immediately and moves on to the next
    /// queued reveal, if any.
    pub fn skip_reveal(&mut self) {
        self.advance_reveal_queue();
    }

    /// Completes the active reveal and every queued reveal immediately,
    /// leaving all messages fully visible. Used when the player submits
    /// input: the log snaps to its final state before the new command.
    pub fn skip_all_reveals(&mut self) {
        self.reveal = None;
        self.reveal_queue.clear();
    }

    fn advance_reveal_queue(&mut self) {
        while let Some(next_index) = self.reveal_queue.pop_front() {
            let is_narration = self
                .messages
                .get(next_index)
                .is_some_and(|msg| msg.kind == MessageKind::Narration);
            if is_narration {
                self.reveal = Some(TypewriterState::new(next_index));
                return;
            }
        }
        self.reveal = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::AppMessage;
    use crate::tui::components::theme::MessageTheme;

    fn app_with_message(kind_normal: bool) -> App {
        let mut app = App::new(false);
        app.messages.clear();
        let theme = MessageTheme;
        let msg = if kind_normal {
            AppMessage::normal("hello world", &theme)
        } else {
            AppMessage::system("hello world", &theme)
        };
        app.messages.push(msg);
        app
    }

    fn app_with_narration_messages(texts: &[&str]) -> App {
        let mut app = App::new(false);
        app.messages.clear();
        let theme = MessageTheme;
        for text in texts {
            app.messages.push(AppMessage::normal(*text, &theme));
        }
        app
    }

    #[test]
    fn start_reveal_arms_for_narration() {
        let mut app = app_with_message(true);
        app.start_reveal(0);
        assert!(app.reveal.is_some());
    }

    #[test]
    fn start_reveal_ignores_non_narration() {
        let mut app = app_with_message(false);
        app.start_reveal(0);
        assert!(app.reveal.is_none());
    }

    #[test]
    fn tick_reveal_advances_and_clears_when_caught_up() {
        let mut app = app_with_message(true);
        app.reveal_rate = 1000.0;
        app.start_reveal(0);
        app.tick_reveal(Duration::from_secs(1));
        assert!(app.reveal.is_none());
    }

    #[test]
    fn tick_reveal_stays_open_while_streaming() {
        let mut app = app_with_message(true);
        app.reveal_rate = 1000.0;
        app.streaming_message_index = Some(0);
        app.start_reveal(0);
        app.tick_reveal(Duration::from_secs(1));
        assert!(app.reveal.is_some());
        assert!(app.reveal.unwrap().is_caught_up(11));
    }

    #[test]
    fn tick_reveal_resumes_once_streaming_finishes() {
        let mut app = app_with_message(true);
        app.reveal_rate = 1000.0;
        app.streaming_message_index = Some(0);
        app.start_reveal(0);
        app.tick_reveal(Duration::from_secs(1));
        assert!(app.reveal.is_some());

        app.streaming_message_index = None;
        app.tick_reveal(Duration::from_secs(1));
        assert!(app.reveal.is_none());
    }

    #[test]
    fn skip_reveal_clears_state() {
        let mut app = app_with_message(true);
        app.start_reveal(0);
        assert!(app.reveal.is_some());
        app.skip_reveal();
        assert!(app.reveal.is_none());
    }

    #[test]
    fn start_reveal_queues_a_second_message_instead_of_stealing_the_reveal() {
        let mut app = app_with_narration_messages(&["you move north.", "a warm tavern."]);
        app.start_reveal(0);
        app.start_reveal(1);

        assert_eq!(app.reveal.unwrap().message_index, 0);
        assert_eq!(app.reveal_queue.len(), 1);
        assert_eq!(app.reveal_queue[0], 1);
    }

    #[test]
    fn completing_current_reveal_starts_the_queued_one() {
        let mut app = app_with_narration_messages(&["you move north.", "a warm tavern."]);
        app.reveal_rate = 1000.0;
        app.start_reveal(0);
        app.start_reveal(1);

        // Finishes message 0 and should immediately begin message 1.
        app.tick_reveal(Duration::from_secs(1));

        assert_eq!(app.reveal.unwrap().message_index, 1);
        assert!(app.reveal_queue.is_empty());
    }

    #[test]
    fn a_burst_of_three_narration_messages_reveals_each_in_turn() {
        let mut app =
            app_with_narration_messages(&["move.", "room description.", "an npc is here."]);
        app.reveal_rate = 1000.0;
        app.start_reveal(0);
        app.start_reveal(1);
        app.start_reveal(2);
        assert_eq!(app.reveal.unwrap().message_index, 0);

        app.tick_reveal(Duration::from_secs(1));
        assert_eq!(app.reveal.unwrap().message_index, 1);

        app.tick_reveal(Duration::from_secs(1));
        assert_eq!(app.reveal.unwrap().message_index, 2);

        app.tick_reveal(Duration::from_secs(1));
        assert!(app.reveal.is_none());
    }

    #[test]
    fn skip_all_reveals_clears_current_and_queue() {
        let mut app = app_with_narration_messages(&["you move north.", "a warm tavern."]);
        app.start_reveal(0);
        app.start_reveal(1);

        app.skip_all_reveals();

        assert!(app.reveal.is_none());
        assert!(app.reveal_queue.is_empty());
    }

    #[test]
    fn skip_reveal_advances_to_the_queued_message() {
        let mut app = app_with_narration_messages(&["you move north.", "a warm tavern."]);
        app.start_reveal(0);
        app.start_reveal(1);

        app.skip_reveal();

        assert_eq!(app.reveal.unwrap().message_index, 1);
        assert!(app.reveal_queue.is_empty());
    }
}
