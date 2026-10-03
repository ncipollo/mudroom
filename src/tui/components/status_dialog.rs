use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Clear, List, ListItem},
};

use crate::tui::components::dialog::centered_rect;

/// A single attribute's current/max value, decoupled from any network or game-domain type so
/// this dialog can be built from whatever entity data a caller has on hand.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeRow {
    pub title: String,
    pub current: i64,
    pub max: i64,
}

/// A reusable "show an entity's full attribute list" popup. Follows the same centered-overlay,
/// closes-on-`Esc` convention as other battle dialogs, but isn't battle-specific itself — any
/// screen can construct one from its own entity data.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusDialog {
    pub entity_name: String,
    pub rows: Vec<AttributeRow>,
}

impl StatusDialog {
    pub fn new(entity_name: String, rows: Vec<AttributeRow>) -> Self {
        Self { entity_name, rows }
    }
}

pub fn render(frame: &mut Frame, dialog: &StatusDialog, area: Rect) {
    let height = (dialog.rows.len() as u16 + 2).min(20);
    let dialog_area = centered_rect(40, height, area);

    frame.render_widget(Clear, dialog_area);

    let items: Vec<ListItem> = dialog
        .rows
        .iter()
        .map(|row| ListItem::new(format!("{:<14} {}/{}", row.title, row.current, row.max)))
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(dialog.entity_name.clone())
            .borders(Borders::ALL),
    );
    frame.render_widget(list, dialog_area);
}
