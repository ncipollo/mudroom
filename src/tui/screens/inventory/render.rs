use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::tui::app::{App, InventoryFocus, InventoryState};
use crate::tui::components::selection::selection_style;

pub fn render(frame: &mut Frame, app: &mut App) {
    let Some(inventory) = &app.inventory else {
        return;
    };

    let areas = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).split(frame.area());
    let cols =
        Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(areas[0]);

    render_equipment_panel(frame, inventory, cols[0]);
    render_bag_panel(frame, inventory, cols[1]);
    render_status_bar(frame, areas[1]);
}

fn render_equipment_panel(frame: &mut Frame, inventory: &InventoryState, area: Rect) {
    let is_focused = inventory.focus == InventoryFocus::Equipment;

    let items: Vec<ListItem> = inventory
        .slots
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            let label = match &slot.equipped {
                Some(item) => format!("{}: {}", slot.slot_name, item.name),
                None => format!("{}: (empty)", slot.slot_name),
            };
            style_item(label, i == inventory.selected_slot_index && is_focused)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("Equipment")
            .borders(Borders::ALL)
            .border_style(focus_style(is_focused)),
    );
    frame.render_widget(list, area);
}

fn render_bag_panel(frame: &mut Frame, inventory: &InventoryState, area: Rect) {
    let is_focused = inventory.focus == InventoryFocus::Bag;

    let items: Vec<ListItem> = inventory
        .bag
        .iter()
        .enumerate()
        .map(|(i, item)| {
            style_item(
                item.name.clone(),
                i == inventory.selected_bag_index && is_focused,
            )
        })
        .collect();

    let title = format!("Bag ({}/{})", inventory.bag.len(), inventory.bag_size);
    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(focus_style(is_focused)),
    );
    frame.render_widget(list, area);
}

fn render_status_bar(frame: &mut Frame, area: Rect) {
    let status = Paragraph::new("↑↓ Navigate  ←→/Tab Switch Pane  Esc Close")
        .style(Style::default().fg(Color::Green))
        .block(Block::default().title("Controls").borders(Borders::ALL));
    frame.render_widget(status, area);
}

fn focus_style(is_focused: bool) -> Style {
    if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    }
}

fn style_item(label: String, selected: bool) -> ListItem<'static> {
    ListItem::new(label).style(selection_style(selected))
}
