use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::tui::app::{App, InventoryFocus, InventoryState};
use crate::tui::components::focus::focus_style;
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
    render_status_bar(frame, inventory, areas[1]);

    if inventory.dialog.is_some() {
        render_item_dialog(frame, inventory, frame.area());
    }
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

fn render_status_bar(frame: &mut Frame, inventory: &InventoryState, area: Rect) {
    let hints = if inventory.dialog.is_some() {
        "↑↓ Navigate  Enter Confirm  Esc Cancel"
    } else {
        "↑↓ Navigate  ←→/Tab Switch Pane  Enter Actions  Esc Close"
    };
    let status = Paragraph::new(hints)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().title("Controls").borders(Borders::ALL));
    frame.render_widget(status, area);
}

fn style_item(label: String, selected: bool) -> ListItem<'static> {
    ListItem::new(label).style(selection_style(selected))
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

fn render_item_dialog(frame: &mut Frame, inventory: &InventoryState, area: Rect) {
    let Some(dialog) = &inventory.dialog else {
        return;
    };

    let height = (dialog.actions.len() as u16 + 2).min(20);
    let dialog_area = centered_rect(30, height, area);

    frame.render_widget(Clear, dialog_area);

    let items: Vec<ListItem> = dialog
        .actions
        .iter()
        .enumerate()
        .map(|(i, action)| {
            ListItem::new(action.label()).style(selection_style(i == dialog.selected_index))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(dialog.item_name.clone())
            .borders(Borders::ALL),
    );
    frame.render_widget(list, dialog_area);
}
