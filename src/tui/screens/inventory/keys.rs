use crossterm::event::{KeyCode, KeyModifiers};

use crate::game::Interaction;
use crate::network::client::send_interaction;
use crate::tui::app::{App, GameMode, ItemAction};

pub async fn handle_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    if app.inventory.as_ref().is_some_and(|i| i.dialog.is_some()) {
        handle_dialog_key(app, modifiers, code).await;
        return;
    }
    match (modifiers, code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => app.should_quit = true,
        (_, KeyCode::Up) => {
            if let Some(inventory) = &mut app.inventory {
                inventory.select_prev();
            }
        }
        (_, KeyCode::Down) => {
            if let Some(inventory) = &mut app.inventory {
                inventory.select_next();
            }
        }
        (_, KeyCode::Left | KeyCode::Right | KeyCode::Tab) => {
            if let Some(inventory) = &mut app.inventory {
                inventory.toggle_focus();
            }
        }
        (_, KeyCode::Enter) => {
            if let Some(inventory) = &mut app.inventory {
                inventory.open_item_dialog();
            }
        }
        (_, KeyCode::Esc) => {
            app.inventory = None;
            app.mode = GameMode::Game;
        }
        _ => {}
    }
}

async fn handle_dialog_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    match (modifiers, code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => app.should_quit = true,
        (_, KeyCode::Up) => {
            if let Some(inventory) = &mut app.inventory {
                inventory.item_dialog_prev();
            }
        }
        (_, KeyCode::Down) => {
            if let Some(inventory) = &mut app.inventory {
                inventory.item_dialog_next();
            }
        }
        (_, KeyCode::Enter) => handle_dialog_confirm(app).await,
        (_, KeyCode::Esc) => {
            if let Some(inventory) = &mut app.inventory {
                inventory.close_item_dialog();
            }
        }
        _ => {}
    }
}

async fn handle_dialog_confirm(app: &mut App) {
    let Some(inventory) = &app.inventory else {
        return;
    };
    let Some(dialog) = &inventory.dialog else {
        return;
    };
    let item_id = dialog.item_id;
    let Some(action) = inventory.selected_action() else {
        return;
    };
    if let (Some(url), Some(client_id)) = (
        app.connection.server_url.as_deref(),
        app.connection.client_id.as_deref(),
    ) {
        let interaction = match action {
            ItemAction::Use => Interaction::UseItem { item_id },
            ItemAction::Equip => Interaction::EquipItem { item_id },
            ItemAction::Unequip => Interaction::UnequipItem { item_id },
            ItemAction::Drop => Interaction::DropItem { item_id },
        };
        let _ = send_interaction(url, client_id, &interaction).await;
    }
    if let Some(inventory) = &mut app.inventory {
        inventory.close_item_dialog();
    }
}
