use std::time::Duration;

use super::BattleState;
use super::log_entry;
use crate::game::engagement::battle::BattleMessage;
use crate::tui::components::theme::MessageTheme;
use crate::tui::components::typewriter::{self, TypewriterState};

impl BattleState {
    /// Converts and appends `messages`, arming the typewriter for any major
    /// update (queuing behind a reveal already in progress) so simultaneous
    /// major messages from the same batch type out one at a time.
    pub fn push_messages(&mut self, messages: Vec<BattleMessage>, theme: &MessageTheme) {
        for message in messages {
            let index = self.message_log.len();
            let is_major = log_entry::is_major(&message);
            self.message_log.push(log_entry::to_entry(message, theme));
            if is_major {
                self.start_reveal(index);
            }
        }
    }

    fn start_reveal(&mut self, index: usize) {
        if self.reveal.is_some() {
            self.reveal_queue.push_back(index);
        } else {
            self.reveal = Some(TypewriterState::new(index));
        }
    }

    pub fn tick_reveal(&mut self, tick: Duration, rate_chars_per_sec: f64) {
        let Some(index) = self.reveal.as_ref().map(|state| state.message_index) else {
            return;
        };
        let Some(entry) = self.message_log.get(index) else {
            self.advance_reveal_queue();
            return;
        };

        let total = typewriter::visible_len(&entry.lines);
        if let Some(state) = self.reveal.as_mut() {
            state.advance(rate_chars_per_sec, tick, total);
            if state.is_caught_up(total) {
                self.advance_reveal_queue();
            }
        }
    }

    fn advance_reveal_queue(&mut self) {
        self.reveal = self.reveal_queue.pop_front().map(TypewriterState::new);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::game::engagement::battle::BattlePhase;
    use crate::network::event::BattleSnapshot;

    fn make_battle() -> BattleState {
        BattleState::new(
            1,
            BattleSnapshot {
                factions: vec!["player".to_string(), "enemy".to_string()],
                participants: HashMap::new(),
                phase: BattlePhase::Cleanup,
                turn_order: vec![],
                countdown_secs: 0,
                max_turn_secs: 30,
                available_abilities: vec![],
            },
        )
    }

    fn ability_cast(name: &str) -> BattleMessage {
        BattleMessage::AbilityCast {
            caster_name: name.to_string(),
            target_name: "Player".to_string(),
            ability_name: "Slash".to_string(),
        }
    }

    #[test]
    fn push_messages_arms_reveal_for_major_message() {
        let mut battle = make_battle();
        let theme = MessageTheme;
        battle.push_messages(vec![ability_cast("Goblin")], &theme);
        assert_eq!(battle.reveal.unwrap().message_index, 0);
    }

    #[test]
    fn push_messages_ignores_low_signal_message() {
        let mut battle = make_battle();
        let theme = MessageTheme;
        battle.push_messages(vec![BattleMessage::Meta("hi".into())], &theme);
        assert!(battle.reveal.is_none());
    }

    #[test]
    fn simultaneous_major_messages_queue_and_reveal_in_turn() {
        let mut battle = make_battle();
        let theme = MessageTheme;
        battle.push_messages(vec![ability_cast("Goblin"), ability_cast("Orc")], &theme);
        assert_eq!(battle.reveal.unwrap().message_index, 0);
        assert_eq!(battle.reveal_queue.len(), 1);

        battle.tick_reveal(Duration::from_secs(1), 1000.0);
        assert_eq!(battle.reveal.unwrap().message_index, 1);
        assert!(battle.reveal_queue.is_empty());

        battle.tick_reveal(Duration::from_secs(1), 1000.0);
        assert!(battle.reveal.is_none());
    }

    #[test]
    fn tick_reveal_advances_and_clears_when_caught_up() {
        let mut battle = make_battle();
        let theme = MessageTheme;
        battle.push_messages(vec![ability_cast("Goblin")], &theme);
        battle.tick_reveal(Duration::from_secs(1), 1000.0);
        assert!(battle.reveal.is_none());
    }
}
