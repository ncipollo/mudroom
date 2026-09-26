use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    text::Text,
    widgets::{Block, Borders, Paragraph},
};

use super::super::components::{cursor, message_log};
use super::{agent_conversation, battle, conversation, inventory, player_select};
use crate::game::{Interaction, Movement, TurnAction};
use crate::tui::app::{App, AppMessage, GameMode};
use crate::tui::commands;

pub async fn handle_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    match (modifiers, code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            app.should_quit = true;
        }
        (_, KeyCode::Char(c)) => {
            app.input.push(c);
            app.command_history.reset_navigation();
        }
        (_, KeyCode::Backspace) => {
            app.input.pop();
            app.command_history.reset_navigation();
        }
        (_, KeyCode::Up) => {
            if let Some(recalled) = app.command_history.prev(&app.input) {
                app.input = recalled;
            }
        }
        (_, KeyCode::Down) => {
            if let Some(recalled) = app.command_history.next() {
                app.input = recalled;
            }
        }
        (_, KeyCode::Enter) => {
            app.skip_all_reveals();
            let input: String = std::mem::take(&mut app.input);
            if input.is_empty() {
                return;
            }
            app.command_history.push(input.clone());
            dispatch_command(app, &input);
            app.messages.push(AppMessage::command(input, &app.theme));
            app.log_scroll.pin_to_bottom();
        }
        _ => {}
    }
}

fn dispatch_command(app: &mut App, input: &str) {
    let cmd = commands::parse(input);
    match cmd {
        commands::Command::Move(direction) => {
            let interaction = Interaction::Movement(Movement::TryDirection(direction));
            app.send_interaction_async(interaction);
        }
        commands::Command::Look(target) => {
            let interaction = match target {
                Some(target) => Interaction::LookAt { target },
                None => Interaction::Look,
            };
            app.send_interaction_async(interaction);
        }
        commands::Command::Help => {
            app.send_interaction_async(Interaction::Help);
        }
        commands::Command::Inventory => {
            app.send_interaction_async(Interaction::OpenInventory);
        }
        commands::Command::Speak(msg) => {
            app.agent_responding |= dispatch_speak(app, msg);
        }
        commands::Command::Take(target) => {
            app.send_interaction_async(Interaction::Take { target });
        }
        commands::Command::Interact { verb, target } => {
            app.send_interaction_async(Interaction::Interact { verb, target });
        }
        commands::Command::Choose(choice) => {
            let action = Interaction::EngagementAction(TurnAction::SelectDialogChoice { choice });
            app.send_interaction_async(action);
        }
        commands::Command::Attack => {
            app.send_interaction_async(Interaction::JoinBattle { engagement_id: 0 });
        }
        _ => {}
    }
}

/// Sends the speak interaction, reporting whether it carried an initial message that was sent.
fn dispatch_speak(app: &App, msg: Option<String>) -> bool {
    let has_initial = msg.is_some();
    let interaction = Interaction::StartConversation {
        initial_message: msg,
    };
    app.send_interaction_async(interaction) && has_initial
}

pub fn render(frame: &mut Frame, app: &mut App) {
    if app.mode == GameMode::PlayerSelect {
        player_select::render(frame, app);
        return;
    }

    if app.mode == GameMode::StandardConversation {
        conversation::render(frame, app);
        return;
    }

    if app.mode == GameMode::AgentConversation {
        agent_conversation::render(frame, app);
        return;
    }

    if app.mode == GameMode::Battle {
        battle::render(frame, app);
        return;
    }

    if app.mode == GameMode::Inventory {
        inventory::render(frame, app);
        return;
    }

    let areas = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(3),
    ])
    .split(frame.area());

    // Message log
    message_log::render(
        frame,
        app,
        Block::default().title("Messages").borders(Borders::ALL),
        areas[0],
    );

    // Status bar
    let status_text = format!(
        "HP: {}/{} | MP: {}/{} | Location: Town Square",
        app.hp_current, app.hp_max, app.mp_current, app.mp_max
    );
    let status =
        Paragraph::new(status_text).block(Block::default().title("Status").borders(Borders::ALL));
    frame.render_widget(status, areas[1]);

    // Input line
    let input_block = Block::default().title("Input").borders(Borders::ALL);
    let input_inner = input_block.inner(areas[2]);
    let input_text = format!("> {}", app.input);
    let input = Paragraph::new(Text::from(input_text)).block(input_block);
    frame.render_widget(input, areas[2]);
    cursor::place_at_end(frame, input_inner, 2, &app.input);
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn press(app: &mut App, code: KeyCode) {
        handle_key(app, KeyModifiers::NONE, code).await;
    }

    async fn type_str(app: &mut App, s: &str) {
        for c in s.chars() {
            press(app, KeyCode::Char(c)).await;
        }
    }

    #[tokio::test]
    async fn up_recalls_last_submitted_command() {
        let mut app = App::new(false);
        type_str(&mut app, "look").await;
        press(&mut app, KeyCode::Enter).await;
        press(&mut app, KeyCode::Up).await;
        assert_eq!(app.input, "look");
    }

    #[tokio::test]
    async fn up_up_down_returns_to_more_recent_entry() {
        let mut app = App::new(false);
        type_str(&mut app, "north").await;
        press(&mut app, KeyCode::Enter).await;
        type_str(&mut app, "look").await;
        press(&mut app, KeyCode::Enter).await;
        press(&mut app, KeyCode::Up).await;
        press(&mut app, KeyCode::Up).await;
        press(&mut app, KeyCode::Down).await;
        assert_eq!(app.input, "look");
    }

    #[tokio::test]
    async fn down_past_newest_restores_pre_navigation_draft() {
        let mut app = App::new(false);
        type_str(&mut app, "look").await;
        press(&mut app, KeyCode::Enter).await;
        type_str(&mut app, "partial").await;
        press(&mut app, KeyCode::Up).await;
        press(&mut app, KeyCode::Down).await;
        assert_eq!(app.input, "partial");
    }

    #[tokio::test]
    async fn editing_after_recall_exits_navigation() {
        let mut app = App::new(false);
        type_str(&mut app, "look").await;
        press(&mut app, KeyCode::Enter).await;
        press(&mut app, KeyCode::Up).await;
        press(&mut app, KeyCode::Char('!')).await;
        press(&mut app, KeyCode::Down).await;
        assert_eq!(app.input, "look!");
    }

    #[tokio::test]
    async fn enter_skips_duplicate_and_empty_history_entries() {
        let mut app = App::new(false);
        type_str(&mut app, "look").await;
        press(&mut app, KeyCode::Enter).await;
        type_str(&mut app, "look").await;
        press(&mut app, KeyCode::Enter).await;
        press(&mut app, KeyCode::Enter).await;
        press(&mut app, KeyCode::Up).await;
        press(&mut app, KeyCode::Up).await;
        assert_eq!(app.input, "look");
    }
}
