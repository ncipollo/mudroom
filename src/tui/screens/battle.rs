use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::game::engagement::battle::{BattleMessage, BattlePhase};
use crate::game::{Interaction, TurnAction};
use crate::network::client::send_interaction;
use crate::network::event::ParticipantInfo;
use crate::tui::app::{App, BattleFocus, BattleState, GameMode};

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

pub fn render(frame: &mut Frame, app: &App) {
    let Some(battle) = &app.battle else {
        return;
    };

    let areas = Layout::vertical([
        Constraint::Length(8),
        Constraint::Fill(1),
        Constraint::Length(3),
    ])
    .split(frame.area());

    render_combatants_panel(frame, battle, areas[0]);
    render_middle_row(frame, app, battle, areas[1]);
    render_status_bar(frame, battle, areas[2]);

    if battle.dialog.is_some() {
        render_target_dialog(frame, battle, frame.area());
    }
}

fn render_combatants_panel(frame: &mut Frame, battle: &BattleState, area: Rect) {
    let is_focused = battle.focus == BattleFocus::EntityList;
    let border_style = if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let outer_block = Block::default()
        .title("Combatants")
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    let cols = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(inner);

    let scroll_y = battle.entity_scroll as u16;

    let left_lines = faction_lines(battle, battle.snapshot.factions.first().map(String::as_str));
    let left = Paragraph::new(Text::from(left_lines)).scroll((scroll_y, 0));
    frame.render_widget(left, cols[0]);

    frame.render_widget(Block::default().borders(Borders::LEFT), cols[1]);

    let right_lines: Vec<Line> = battle
        .snapshot
        .factions
        .iter()
        .skip(1)
        .flat_map(|f| faction_lines(battle, Some(f.as_str())))
        .collect();
    let right = Paragraph::new(Text::from(right_lines)).scroll((scroll_y, 0));
    frame.render_widget(right, cols[2]);
}

fn faction_lines(battle: &BattleState, faction: Option<&str>) -> Vec<Line<'static>> {
    let Some(faction_name) = faction else {
        return vec![];
    };
    let mut lines = vec![Line::from(Span::styled(
        faction_name.to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    ))];
    if let Some(participants) = battle.snapshot.participants.get(faction_name) {
        for p in participants {
            lines.push(participant_line(p));
        }
    }
    lines
}

fn participant_line(p: &ParticipantInfo) -> Line<'static> {
    let hp_ratio = if p.hp_max > 0 {
        (p.hp_current as f64 / p.hp_max as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let bar_width = 8usize;
    let filled = (hp_ratio * bar_width as f64).round() as usize;
    let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(bar_width - filled));
    let hp_color = if hp_ratio > 0.5 {
        Color::Green
    } else if hp_ratio > 0.25 {
        Color::Yellow
    } else {
        Color::Red
    };
    Line::from(vec![
        Span::raw(format!("{:<12} ", p.name)),
        Span::styled(bar, Style::default().fg(hp_color)),
        Span::raw(format!(" {}/{}", p.hp_current, p.hp_max)),
    ])
}

fn render_middle_row(frame: &mut Frame, app: &App, battle: &BattleState, area: Rect) {
    let cols = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);

    render_abilities_panel(frame, battle, cols[0]);
    render_message_log(frame, app, battle, cols[1]);
}

fn render_abilities_panel(frame: &mut Frame, battle: &BattleState, area: Rect) {
    let is_focused = battle.focus == BattleFocus::Abilities;
    let border_style = if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let items: Vec<ListItem> = battle
        .snapshot
        .available_abilities
        .iter()
        .enumerate()
        .map(|(i, ability)| {
            let cost_str = ability
                .costs
                .iter()
                .map(|c| {
                    let crate::game::component::Cost::Resource {
                        resource_id,
                        amount,
                    } = c;
                    format!("{resource_id}:{amount}")
                })
                .collect::<Vec<_>>()
                .join(", ");
            let label = if cost_str.is_empty() {
                ability.name.clone()
            } else {
                format!("{} ({})", ability.name, cost_str)
            };
            if i == battle.selected_ability_index && is_focused {
                ListItem::new(label).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ListItem::new(label)
            }
        })
        .collect();

    let title = if battle.is_player_turn() {
        "Abilities"
    } else {
        "Abilities (waiting\u{2026})"
    };
    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style),
    );
    frame.render_widget(list, area);
}

fn render_message_log(frame: &mut Frame, app: &App, battle: &BattleState, area: Rect) {
    let panel_height = area.height.saturating_sub(2) as usize;

    let all_lines: Vec<Line> = battle
        .message_log
        .iter()
        .map(|msg| battle_message_to_line(msg))
        .collect();

    let total_lines = all_lines.len();
    let max_offset = total_lines.saturating_sub(panel_height);
    let effective_offset = app.scroll_offset.min(max_offset);
    let scroll_from_top = max_offset.saturating_sub(effective_offset) as u16;

    let log = Paragraph::new(Text::from(all_lines))
        .block(Block::default().title("Battle Log").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
        .scroll((scroll_from_top, 0));
    frame.render_widget(log, area);
}

fn battle_message_to_line(msg: &BattleMessage) -> Line<'_> {
    match msg {
        BattleMessage::PhaseChange { .. } => Line::from(Span::styled(
            msg.to_string(),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC | Modifier::DIM),
        )),
        BattleMessage::AbilityCast { .. } => Line::from(Span::styled(
            msg.to_string(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        BattleMessage::EntityDied { .. } => Line::from(Span::styled(
            msg.to_string(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        BattleMessage::Meta(_) => Line::from(Span::styled(
            msg.to_string(),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )),
    }
}

fn render_status_bar(frame: &mut Frame, battle: &BattleState, area: Rect) {
    let phase_str = battle.snapshot.phase.to_string();

    let player_faction = battle.snapshot.factions.first();
    let player_hp = player_faction
        .and_then(|f| battle.snapshot.participants.get(f))
        .and_then(|ps| ps.first());

    let hints = if battle.dialog.is_some() {
        "↑↓ Navigate  Enter Confirm  Esc Cancel".to_string()
    } else {
        "↑↓ Navigate  Tab Switch Focus  Enter Select Ability  Esc Leave".to_string()
    };

    let timer_str = match battle.snapshot.phase {
        BattlePhase::AttackerPlanning | BattlePhase::DefenderResponse => {
            let remaining = battle.snapshot.countdown_ticks;
            let max = battle.snapshot.max_turn_ticks.max(1);
            let filled = ((remaining * 10) / max).min(10) as usize;
            let bar = format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled));
            format!(" | [{bar}] {remaining}")
        }
        _ => String::new(),
    };

    let status_text = if let Some(p) = player_hp {
        format!(
            "HP: {}/{}{timer_str} | [{phase_str}] | {hints}",
            p.hp_current, p.hp_max
        )
    } else {
        format!("[{phase_str}]{timer_str} | {hints}")
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Green))
        .block(Block::default().title("Status").borders(Borders::ALL));
    frame.render_widget(status, area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

fn render_target_dialog(frame: &mut Frame, battle: &BattleState, area: Rect) {
    let Some(dialog) = &battle.dialog else { return };

    let height = (dialog.targets.len() as u16 + 2).min(20);
    let dialog_area = centered_rect(40, height, area);

    frame.render_widget(Clear, dialog_area);

    let items: Vec<ListItem> = dialog
        .targets
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if i == dialog.selected_index {
                ListItem::new(p.name.clone()).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ListItem::new(p.name.clone())
            }
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("Select Target")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(list, dialog_area);
}
