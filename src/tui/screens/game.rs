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
        (_, KeyCode::Char('i')) if app.input.is_empty() => {
            app.send_interaction_async(Interaction::OpenInventory);
        }
        (_, KeyCode::Char(c)) => {
            app.input.push(c);
        }
        (_, KeyCode::Backspace) => {
            app.input.pop();
        }
        (_, KeyCode::Enter) => {
            app.skip_all_reveals();
            let input: String = std::mem::take(&mut app.input);
            if input.is_empty() {
                return;
            }
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
        commands::Command::Speak(msg) => {
            app.agent_responding |= dispatch_speak(app, msg);
        }
        commands::Command::Take(target) => {
            app.send_interaction_async(Interaction::Take { target });
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
