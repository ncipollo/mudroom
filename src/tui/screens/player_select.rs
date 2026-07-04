use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::network::client::{create_player, list_classes, select_player};
use crate::tui::app::{App, GameMode};

pub async fn handle_key(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    if modifiers == KeyModifiers::CONTROL && code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    if app.player_select.class_select.active {
        handle_class_select_key(app, code).await;
        return;
    }

    if app.player_select.creating_player {
        handle_name_input_key(app, code).await;
        return;
    }

    match code {
        KeyCode::Up => app.select_prev(),
        KeyCode::Down => app.select_next(),
        KeyCode::Enter => {
            let create_idx = app.player_select.players.len();
            if app.player_select.selected_index == create_idx {
                app.start_create();
            } else if let Some(player) = app
                .player_select
                .players
                .get(app.player_select.selected_index)
            {
                let player_id = player.id;
                if let (Some(url), Some(client_id)) = (
                    app.connection.server_url.clone(),
                    app.connection.client_id.clone(),
                ) && select_player(&url, &client_id, player_id).await.is_ok()
                {
                    app.mode = GameMode::Game;
                }
            }
        }
        _ => {}
    }
}

async fn handle_name_input_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.cancel_create(),
        KeyCode::Backspace => {
            app.player_select.player_name_input.pop();
        }
        KeyCode::Enter => {
            let name = app.player_select.player_name_input.trim().to_string();
            if !name.is_empty()
                && let (Some(url), Some(client_id)) = (
                    app.connection.server_url.clone(),
                    app.connection.client_id.clone(),
                )
                && let Ok(classes) = list_classes(&url).await
            {
                if classes.is_empty() {
                    create_and_select(app, &url, &client_id, &name, None).await;
                } else {
                    app.start_class_select(classes);
                }
            }
        }
        KeyCode::Char(c) => app.player_select.player_name_input.push(c),
        _ => {}
    }
}

async fn handle_class_select_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.cancel_class_select(),
        KeyCode::Up => app.class_select_prev(),
        KeyCode::Down => app.class_select_next(),
        KeyCode::Enter => {
            if let (Some(url), Some(client_id)) = (
                app.connection.server_url.clone(),
                app.connection.client_id.clone(),
            ) {
                let name = app.player_select.player_name_input.trim().to_string();
                let class_id = app
                    .player_select
                    .class_select
                    .classes
                    .get(app.player_select.class_select.selected_index)
                    .map(|c| c.id.clone());
                if let Some(class_id) = class_id {
                    create_and_select(app, &url, &client_id, &name, Some(&class_id)).await;
                }
            }
        }
        _ => {}
    }
}

async fn create_and_select(
    app: &mut App,
    url: &str,
    client_id: &str,
    name: &str,
    class_id: Option<&str>,
) {
    if let Ok(info) = create_player(url, client_id, name, class_id).await {
        let player_id = info.id;
        app.player_select.players.push(info);
        app.cancel_create();
        if select_player(url, client_id, player_id).await.is_ok() {
            app.mode = GameMode::Game;
        }
    }
}

pub fn render(frame: &mut Frame, app: &App) {
    if app.player_select.class_select.active {
        render_class_select(frame, app);
    } else {
        render_player_select(frame, app);
    }
}

fn render_class_select(frame: &mut Frame, app: &App) {
    let areas = Layout::vertical([Constraint::Fill(1), Constraint::Length(5)]).split(frame.area());

    let title = "Select a Class (↑↓ to navigate, Enter to select, Esc to go back)";
    let items: Vec<ListItem> = app
        .player_select
        .class_select
        .classes
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let style = if i == app.player_select.class_select.selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(c.name.clone(), style)))
        })
        .collect();

    let list = List::new(items).block(Block::default().title(title).borders(Borders::ALL));
    frame.render_widget(list, areas[0]);

    let description = app
        .player_select
        .class_select
        .classes
        .get(app.player_select.class_select.selected_index)
        .and_then(|c| c.description.as_deref())
        .unwrap_or("No description.");
    let desc_widget = Paragraph::new(description)
        .block(Block::default().title("Description").borders(Borders::ALL));
    frame.render_widget(desc_widget, areas[1]);
}

fn render_player_select(frame: &mut Frame, app: &App) {
    let areas = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).split(frame.area());

    let title = "Select a Player (↑↓ to navigate, Enter to select, Esc to cancel create)";
    let mut items: Vec<ListItem> = app
        .player_select
        .players
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style =
                if i == app.player_select.selected_index && !app.player_select.creating_player {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
            ListItem::new(Line::from(Span::styled(p.name.clone(), style)))
        })
        .collect();

    let create_idx = app.player_select.players.len();
    let create_style =
        if app.player_select.selected_index == create_idx && !app.player_select.creating_player {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
    items.push(ListItem::new(Line::from(Span::styled(
        "[ Create New Player ]",
        create_style,
    ))));

    let list = List::new(items).block(Block::default().title(title).borders(Borders::ALL));
    frame.render_widget(list, areas[0]);

    let input_text = if app.player_select.creating_player {
        format!("Name: {}_", app.player_select.player_name_input)
    } else {
        String::new()
    };
    let input_title = if app.player_select.creating_player {
        "Enter player name"
    } else {
        "Input"
    };
    let input = Paragraph::new(input_text)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().title(input_title).borders(Borders::ALL));
    frame.render_widget(input, areas[1]);
}
