use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    text::Text,
    widgets::{Block, Borders, Paragraph},
};

use super::super::components::message_log;
use super::{agent_conversation, battle, conversation, inventory, player_select};
use crate::game::{Interaction, Movement, TurnAction};
use crate::network::client::send_interaction;
use crate::tui::app::{App, AppMessage, GameMode};
use crate::tui::commands;

pub async fn handle_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    match (modifiers, code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            app.should_quit = true;
        }
        (_, KeyCode::Char('i')) if app.input.is_empty() => {
            let url = app.connection.server_url.as_deref();
            let client_id = app.connection.client_id.as_deref();
            send(url, client_id, &Interaction::OpenInventory).await;
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
            dispatch_command(app, &input).await;
            app.messages.push(AppMessage::command(input, &app.theme));
            app.log_scroll.pin_to_bottom();
        }
        _ => {}
    }
}

async fn dispatch_command(app: &mut App, input: &str) {
    let cmd = commands::parse(input);
    let url = app.connection.server_url.as_deref();
    let client_id = app.connection.client_id.as_deref();
    match cmd {
        commands::Command::Move(direction) => {
            let interaction = Interaction::Movement(Movement::TryDirection(direction));
            send(url, client_id, &interaction).await;
        }
        commands::Command::Look(target) => {
            let interaction = match target {
                Some(target) => Interaction::LookAt { target },
                None => Interaction::Look,
            };
            send(url, client_id, &interaction).await;
        }
        commands::Command::Help => {
            send(url, client_id, &Interaction::Help).await;
        }
        commands::Command::Speak(msg) => {
            app.agent_responding |= dispatch_speak(url, client_id, msg).await;
        }
        commands::Command::Take(target) => {
            send(url, client_id, &Interaction::Take { target }).await;
        }
        commands::Command::Choose(choice) => {
            let action = Interaction::EngagementAction(TurnAction::SelectDialogChoice { choice });
            send(url, client_id, &action).await;
        }
        commands::Command::Attack => {
            send(
                url,
                client_id,
                &Interaction::JoinBattle { engagement_id: 0 },
            )
            .await;
        }
        _ => {}
    }
}

/// Sends `interaction` when both connection details are present, reporting whether it did.
async fn send(url: Option<&str>, client_id: Option<&str>, interaction: &Interaction) -> bool {
    let (Some(url), Some(client_id)) = (url, client_id) else {
        return false;
    };
    let _ = send_interaction(url, client_id, interaction).await;
    true
}

/// Sends the speak interaction, reporting whether it carried an initial message that was sent.
async fn dispatch_speak(url: Option<&str>, client_id: Option<&str>, msg: Option<String>) -> bool {
    let has_initial = msg.is_some();
    let interaction = Interaction::StartConversation {
        initial_message: msg,
    };
    send(url, client_id, &interaction).await && has_initial
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
    let status = Paragraph::new("HP: 100 | MP: 50 | Location: Town Square")
        .block(Block::default().title("Status").borders(Borders::ALL));
    frame.render_widget(status, areas[1]);

    // Input line
    let input_text = format!("> {}", app.input);
    let input = Paragraph::new(Text::from(input_text))
        .block(Block::default().title("Input").borders(Borders::ALL));
    frame.render_widget(input, areas[2]);
}
