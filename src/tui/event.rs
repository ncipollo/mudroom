use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers, MouseEventKind};
use futures_util::StreamExt;
use futures_util::future::poll_immediate;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;
use tokio::time;

use crate::network::NetworkEvent;
use crate::network::client::list_players;

use super::app::{App, BattleFocus, GameMode};
use super::components::scroll::{KEY_SCROLL_LINES, MOUSE_SCROLL_LINES, ScrollState};
use super::screens::{
    agent_conversation, battle as battle_screen, conversation, game as game_screen, player_select,
};

const REVEAL_TICK: Duration = Duration::from_millis(50);

pub async fn run(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    mut net_rx: mpsc::Receiver<NetworkEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut event_stream = EventStream::new();
    let mut spinner_ticker = time::interval(Duration::from_millis(100));
    spinner_ticker.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
    let mut reveal_ticker = time::interval(REVEAL_TICK);
    reveal_ticker.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    load_initial_players(app).await;

    while !app.should_quit {
        terminal.draw(|frame| game_screen::render(frame, app))?;

        tokio::select! {
            _ = spinner_ticker.tick(), if app.agent_responding => {}
            _ = reveal_ticker.tick(), if app.reveal.is_some() || app.battle.as_ref().is_some_and(|b| b.reveal.is_some()) => {
                app.tick_reveal(REVEAL_TICK);
                let rate = app.reveal_rate;
                if let Some(battle) = &mut app.battle {
                    battle.tick_reveal(REVEAL_TICK, rate);
                }
            }
            maybe_event = event_stream.next() => {
                if !handle_terminal_events(app, &mut event_stream, maybe_event).await {
                    break;
                }
            }
            maybe_net = net_rx.recv() => {
                if let Some(event) = maybe_net {
                    app.handle_network_event(event);
                }
            }
        }
    }

    Ok(())
}

/// Loads the player list immediately when starting in PlayerSelect mode.
async fn load_initial_players(app: &mut App) {
    if app.mode == GameMode::PlayerSelect
        && let (Some(url), Some(client_id)) = (
            app.connection.server_url.clone(),
            app.connection.client_id.clone(),
        )
        && let Ok(players) = list_players(&url, &client_id).await
    {
        app.player_select.players = players;
    }
}

/// Handles a terminal event, then drains any already-queued events (e.g. a
/// trackpad momentum burst) before the next redraw, so a flick coalesces into
/// one frame instead of one full redraw per wheel tick. Returns `false` if
/// the event loop should exit.
///
/// The drain must poll with the real task waker (`poll_immediate`), not a
/// no-op one like `now_or_never`: when the stream is empty, crossterm arms
/// its background wake task with the waker from that poll and ignores wakers
/// from later polls, so a dead waker would leave the loop deaf to input.
async fn handle_terminal_events(
    app: &mut App,
    event_stream: &mut EventStream,
    first: Option<Result<Event, std::io::Error>>,
) -> bool {
    if !handle_terminal_event(app, first).await {
        return false;
    }
    while let Some(pending) = poll_immediate(event_stream.next()).await {
        if !handle_terminal_event(app, pending).await {
            return false;
        }
    }
    true
}

/// Dispatches a terminal event. Returns `false` if the event loop should exit.
async fn handle_terminal_event(
    app: &mut App,
    maybe_event: Option<Result<Event, std::io::Error>>,
) -> bool {
    match maybe_event {
        Some(Ok(Event::Key(key))) => {
            if handle_scroll_key(app, key.modifiers, key.code) {
                return true;
            }
            match app.mode {
                GameMode::PlayerSelect => {
                    player_select::handle_key(app, key.modifiers, key.code).await;
                }
                GameMode::Game => {
                    game_screen::handle_key(app, key.modifiers, key.code).await;
                }
                GameMode::StandardConversation => {
                    conversation::handle_key(app, key.modifiers, key.code).await;
                }
                GameMode::AgentConversation => {
                    agent_conversation::handle_key(app, key.modifiers, key.code).await;
                }
                GameMode::Battle => {
                    battle_screen::handle_key(app, key.modifiers, key.code).await;
                }
            }
            true
        }
        Some(Ok(Event::Mouse(mouse))) => {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    if let Some(state) = scroll_target(app) {
                        state.scroll_up(MOUSE_SCROLL_LINES);
                    }
                }
                MouseEventKind::ScrollDown => {
                    if let Some(state) = scroll_target(app) {
                        state.scroll_down(MOUSE_SCROLL_LINES);
                    }
                }
                _ => {}
            }
            true
        }
        Some(Ok(_)) => true,
        Some(Err(_)) | None => false,
    }
}

/// The scroll state for the message log visible in the current mode.
fn scroll_target(app: &mut App) -> Option<&mut ScrollState> {
    match app.mode {
        GameMode::PlayerSelect => None,
        GameMode::Battle => app.battle.as_mut().map(|battle| &mut battle.log_scroll),
        _ => Some(&mut app.log_scroll),
    }
}

/// Handles the shared scroll key bindings. Returns `true` if the key was a
/// scroll binding and has been consumed.
///
/// Shift+Up/Down and Ctrl+Up/Down scroll by line (both are bound because
/// terminals differ in which modifier+arrow combinations they report);
/// PageUp/PageDown scroll by a full viewport.
fn handle_scroll_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) -> bool {
    let line_scroll =
        modifiers.contains(KeyModifiers::SHIFT) || modifiers.contains(KeyModifiers::CONTROL);
    match code {
        KeyCode::Up if line_scroll => {
            if let Some(state) = scroll_target(app) {
                state.scroll_up(KEY_SCROLL_LINES);
            }
            true
        }
        KeyCode::Down if line_scroll => {
            if let Some(state) = scroll_target(app) {
                state.scroll_down(KEY_SCROLL_LINES);
            }
            true
        }
        KeyCode::PageUp => {
            handle_page_scroll(app, true);
            true
        }
        KeyCode::PageDown => {
            handle_page_scroll(app, false);
            true
        }
        _ => false,
    }
}

/// PageUp/PageDown page the visible log, except in battle with the combatants
/// panel focused, where they scroll that panel (clamped during render).
fn handle_page_scroll(app: &mut App, up: bool) {
    if app.mode == GameMode::Battle
        && let Some(battle) = &mut app.battle
        && battle.focus == BattleFocus::EntityList
    {
        if up {
            battle.entity_scroll = battle.entity_scroll.saturating_sub(1);
        } else {
            battle.entity_scroll += 1;
        }
        return;
    }
    if let Some(state) = scroll_target(app) {
        if up {
            state.page_up();
        } else {
            state.page_down();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::game::engagement::battle::BattlePhase;
    use crate::network::event::BattleSnapshot;
    use crate::tui::app::BattleState;

    fn battle_app() -> App {
        let mut app = App::new(false);
        app.mode = GameMode::Battle;
        app.battle = Some(BattleState::new(
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
        ));
        app
    }

    #[test]
    fn scroll_target_is_none_in_player_select() {
        let mut app = App::new(false);
        app.mode = GameMode::PlayerSelect;
        assert!(scroll_target(&mut app).is_none());
    }

    #[test]
    fn scroll_target_is_app_log_in_game_mode() {
        let mut app = App::new(false);
        app.log_scroll.sync(100, 10);
        scroll_target(&mut app).unwrap().scroll_up(2);
        assert_eq!(app.log_scroll.offset(), 2);
    }

    #[test]
    fn scroll_target_is_battle_log_in_battle_mode() {
        let mut app = battle_app();
        if let Some(battle) = &mut app.battle {
            battle.log_scroll.sync(100, 10);
        }
        scroll_target(&mut app).unwrap().scroll_up(2);
        assert_eq!(app.battle.unwrap().log_scroll.offset(), 2);
        assert_eq!(app.log_scroll.offset(), 0);
    }

    #[test]
    fn shift_up_scrolls_one_line() {
        let mut app = App::new(false);
        app.log_scroll.sync(100, 10);
        assert!(handle_scroll_key(
            &mut app,
            KeyModifiers::SHIFT,
            KeyCode::Up
        ));
        assert_eq!(app.log_scroll.offset(), 1);
    }

    #[test]
    fn ctrl_down_scrolls_back_one_line() {
        let mut app = App::new(false);
        app.log_scroll.sync(100, 10);
        app.log_scroll.scroll_up(5);
        assert!(handle_scroll_key(
            &mut app,
            KeyModifiers::CONTROL,
            KeyCode::Down
        ));
        assert_eq!(app.log_scroll.offset(), 4);
    }

    #[test]
    fn page_up_scrolls_a_full_viewport() {
        let mut app = App::new(false);
        app.log_scroll.sync(100, 10);
        assert!(handle_scroll_key(
            &mut app,
            KeyModifiers::NONE,
            KeyCode::PageUp
        ));
        assert_eq!(app.log_scroll.offset(), 10);
    }

    #[test]
    fn plain_arrows_and_chars_are_not_scroll_keys() {
        let mut app = App::new(false);
        assert!(!handle_scroll_key(
            &mut app,
            KeyModifiers::NONE,
            KeyCode::Up
        ));
        assert!(!handle_scroll_key(
            &mut app,
            KeyModifiers::NONE,
            KeyCode::Down
        ));
        assert!(!handle_scroll_key(
            &mut app,
            KeyModifiers::SHIFT,
            KeyCode::Char('A')
        ));
    }

    #[test]
    fn page_keys_scroll_entity_panel_when_entity_list_focused() {
        let mut app = battle_app();
        if let Some(battle) = &mut app.battle {
            battle.focus = BattleFocus::EntityList;
        }
        assert!(handle_scroll_key(
            &mut app,
            KeyModifiers::NONE,
            KeyCode::PageDown
        ));
        assert_eq!(app.battle.as_ref().unwrap().entity_scroll, 1);
        assert!(handle_scroll_key(
            &mut app,
            KeyModifiers::NONE,
            KeyCode::PageUp
        ));
        assert_eq!(app.battle.as_ref().unwrap().entity_scroll, 0);
    }
}
