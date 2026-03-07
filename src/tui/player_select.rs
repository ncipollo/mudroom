use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use super::app::App;

pub fn render(frame: &mut Frame, app: &App) {
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
