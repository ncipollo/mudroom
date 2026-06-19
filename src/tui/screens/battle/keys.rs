use crossterm::event::{KeyCode, KeyModifiers};

use crate::game::{Interaction, TurnAction};
use crate::network::client::send_interaction;
use crate::tui::app::{App, BattleFocus, GameMode};

pub async fn handle_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    if app.battle.as_ref().is_some_and(|b| b.dialog.is_some()) {
        handle_dialog_key(app, modifiers, code).await;
        return;
    }
    match (modifiers, code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => app.should_quit = true,
        (_, KeyCode::Up) => handle_navigate_up(app),
        (_, KeyCode::Down) => handle_navigate_down(app),
        (_, KeyCode::Tab) => {
            if let Some(battle) = &mut app.battle {
                battle.toggle_focus();
            }
        }
        (_, KeyCode::Enter) => handle_ability_selected(app),
        (_, KeyCode::Esc) => handle_leave_battle(app).await,
        (_, KeyCode::PageUp) => handle_page_up(app),
        (_, KeyCode::PageDown) => handle_page_down(app),
        _ => {}
    }
}

async fn handle_dialog_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    match (modifiers, code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => app.should_quit = true,
        (_, KeyCode::Up) => {
            if let Some(battle) = &mut app.battle {
                battle.target_dialog_prev();
            }
        }
        (_, KeyCode::Down) => {
            if let Some(battle) = &mut app.battle {
                battle.target_dialog_next();
            }
        }
        (_, KeyCode::Enter) => handle_dialog_confirm(app).await,
        (_, KeyCode::Esc) => {
            if let Some(battle) = &mut app.battle {
                battle.close_target_dialog();
            }
        }
        _ => {}
    }
}

fn handle_navigate_up(app: &mut App) {
    if let Some(battle) = &mut app.battle {
        match battle.focus {
            BattleFocus::Abilities => battle.select_prev_ability(),
            BattleFocus::EntityList => battle.select_prev_entity(),
        }
    }
}

fn handle_navigate_down(app: &mut App) {
    if let Some(battle) = &mut app.battle {
        match battle.focus {
            BattleFocus::Abilities => battle.select_next_ability(),
            BattleFocus::EntityList => battle.select_next_entity(),
        }
    }
}

fn handle_ability_selected(app: &mut App) {
    let Some(battle) = &mut app.battle else {
        return;
    };
    if !battle.is_player_turn() {
        return;
    }
    if battle.focus != BattleFocus::Abilities {
        return;
    }
    let ability_id = battle
        .snapshot
        .available_abilities
        .get(battle.selected_ability_index)
        .map(|a| a.id.clone());
    if let Some(ability_id) = ability_id {
        battle.open_target_dialog(ability_id);
    }
}

async fn handle_dialog_confirm(app: &mut App) {
    let Some(battle) = &app.battle else { return };
    let Some(dialog) = &battle.dialog else { return };
    let ability_id = dialog.pending_ability_id.clone();
    let target_id = battle.dialog_target_id();
    let Some(url) = app.connection.server_url.as_deref() else {
        if let Some(battle) = &mut app.battle {
            battle.close_target_dialog();
        }
        return;
    };
    let url = url.to_owned();
    let Some(client_id) = app.connection.client_id.as_deref() else {
        if let Some(battle) = &mut app.battle {
            battle.close_target_dialog();
        }
        return;
    };
    let client_id = client_id.to_owned();
    if let Some(target_id) = target_id {
        let action = Interaction::EngagementAction(TurnAction::QueueAbility {
            ability_id,
            target_id,
        });
        let _ = send_interaction(&url, &client_id, &action).await;
    }
    if let Some(battle) = &mut app.battle {
        battle.close_target_dialog();
    }
}

async fn handle_leave_battle(app: &mut App) {
    if let Some(battle) = &app.battle {
        let engagement_id = battle.engagement_id;
        if let (Some(url), Some(client_id)) = (
            app.connection.server_url.as_deref(),
            app.connection.client_id.as_deref(),
        ) {
            let _ =
                send_interaction(url, client_id, &Interaction::LeaveBattle { engagement_id }).await;
        }
    }
    app.battle = None;
    app.mode = GameMode::Game;
}

fn handle_page_up(app: &mut App) {
    if app
        .battle
        .as_ref()
        .is_some_and(|b| b.focus == BattleFocus::EntityList)
    {
        if let Some(battle) = &mut app.battle {
            battle.entity_scroll = battle.entity_scroll.saturating_sub(1);
        }
    } else {
        app.scroll_up();
    }
}

fn handle_page_down(app: &mut App) {
    if app
        .battle
        .as_ref()
        .is_some_and(|b| b.focus == BattleFocus::EntityList)
    {
        if let Some(battle) = &mut app.battle {
            battle.entity_scroll += 1;
        }
    } else {
        app.scroll_down();
    }
}
