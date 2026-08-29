use crate::network::event::InventoryItemInfo;

/// Popup state for picking a bag item to equip into a specific equipment slot.
/// Opened from the equipment pane (empty slot or the "Swap" action on an occupied
/// slot); the item list is pre-filtered to the item types the slot accepts.
#[derive(Debug, Clone)]
pub struct SlotPickerDialog {
    pub slot_name: String,
    pub items: Vec<InventoryItemInfo>,
    pub selected_index: usize,
}

impl SlotPickerDialog {
    pub fn new(slot_name: String, items: Vec<InventoryItemInfo>) -> Self {
        Self {
            slot_name,
            items,
            selected_index: 0,
        }
    }

    pub fn next(&mut self) {
        let len = self.items.len();
        if len > 0 {
            self.selected_index = (self.selected_index + 1) % len;
        }
    }

    pub fn prev(&mut self) {
        let len = self.items.len();
        if len > 0 {
            self.selected_index = (self.selected_index + len - 1) % len;
        }
    }

    pub fn selected_item_id(&self) -> Option<i64> {
        self.items.get(self.selected_index).map(|item| item.item_id)
    }
}
