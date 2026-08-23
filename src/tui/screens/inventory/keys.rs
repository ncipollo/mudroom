use crossterm::event::{KeyCode, KeyModifiers};

use crate::tui::app::{App, GameMode};

pub async fn handle_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
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
        (_, KeyCode::Esc) => {
            app.inventory = None;
            app.mode = GameMode::Game;
        }
        _ => {}
    }
}
