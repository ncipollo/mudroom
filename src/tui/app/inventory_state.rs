mod slot_picker;

pub use slot_picker::SlotPickerDialog;

use crate::network::event::{InventoryItemInfo, InventorySlotInfo};

#[derive(Debug, Clone, PartialEq)]
pub enum InventoryFocus {
    Equipment,
    Bag,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ItemAction {
    Use,
    Equip,
    Swap,
    Unequip,
    Drop,
}

impl ItemAction {
    pub fn label(self) -> &'static str {
        match self {
            ItemAction::Use => "Use",
            ItemAction::Equip => "Equip",
            ItemAction::Swap => "Swap",
            ItemAction::Unequip => "Unequip",
            ItemAction::Drop => "Drop",
        }
    }
}

/// Bag items whose `item_type` matches one of the types the slot accepts.
fn eligible_bag_items(
    bag: &[InventoryItemInfo],
    slot: &InventorySlotInfo,
) -> Vec<InventoryItemInfo> {
    bag.iter()
        .filter(|item| slot.item_types.iter().any(|t| t == &item.item_type))
        .cloned()
        .collect()
}

#[derive(Debug, Clone)]
pub struct ItemActionDialog {
    pub item_id: i64,
    pub item_name: String,
    pub actions: Vec<ItemAction>,
    pub selected_index: usize,
}

#[derive(Debug, Clone)]
pub struct InventoryState {
    pub slots: Vec<InventorySlotInfo>,
    pub bag: Vec<InventoryItemInfo>,
    pub bag_size: usize,
    pub focus: InventoryFocus,
    pub selected_slot_index: usize,
    pub selected_bag_index: usize,
    pub dialog: Option<ItemActionDialog>,
    pub slot_picker: Option<SlotPickerDialog>,
}

impl InventoryState {
    pub fn new(
        slots: Vec<InventorySlotInfo>,
        bag: Vec<InventoryItemInfo>,
        bag_size: usize,
    ) -> Self {
        Self {
            slots,
            bag,
            bag_size,
            focus: InventoryFocus::Equipment,
            selected_slot_index: 0,
            selected_bag_index: 0,
            dialog: None,
            slot_picker: None,
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            InventoryFocus::Equipment => InventoryFocus::Bag,
            InventoryFocus::Bag => InventoryFocus::Equipment,
        };
    }

    pub fn select_next(&mut self) {
        match self.focus {
            InventoryFocus::Equipment => {
                let len = self.slots.len();
                if len > 0 {
                    self.selected_slot_index = (self.selected_slot_index + 1) % len;
                }
            }
            InventoryFocus::Bag => {
                let len = self.bag.len();
                if len > 0 {
                    self.selected_bag_index = (self.selected_bag_index + 1) % len;
                }
            }
        }
    }

    pub fn select_prev(&mut self) {
        match self.focus {
            InventoryFocus::Equipment => {
                let len = self.slots.len();
                if len > 0 {
                    self.selected_slot_index = (self.selected_slot_index + len - 1) % len;
                }
            }
            InventoryFocus::Bag => {
                let len = self.bag.len();
                if len > 0 {
                    self.selected_bag_index = (self.selected_bag_index + len - 1) % len;
                }
            }
        }
    }

    /// Opens the item-action dialog for the currently focused+selected item. When the equipment
    /// pane is focused on an empty slot this opens the slot-item picker instead. A no-op when the
    /// bag is empty, or the empty slot has no eligible bag items.
    pub fn open_item_dialog(&mut self) {
        if self.focus == InventoryFocus::Equipment
            && self
                .slots
                .get(self.selected_slot_index)
                .is_some_and(|slot| slot.equipped.is_none())
        {
            self.open_slot_picker();
            return;
        }
        let Some((item_id, item_name, actions)) = self.dialog_target() else {
            return;
        };
        self.dialog = Some(ItemActionDialog {
            item_id,
            item_name,
            actions,
            selected_index: 0,
        });
    }

    /// Opens the slot-item picker for the currently selected equipment slot, listing bag items
    /// whose type fits the slot. Closes any open action dialog. A no-op when no bag item fits.
    pub fn open_slot_picker(&mut self) {
        let Some(slot) = self.slots.get(self.selected_slot_index) else {
            return;
        };
        let items = eligible_bag_items(&self.bag, slot);
        if items.is_empty() {
            return;
        }
        self.dialog = None;
        self.slot_picker = Some(SlotPickerDialog::new(slot.slot_name.clone(), items));
    }

    pub fn close_slot_picker(&mut self) {
        self.slot_picker = None;
    }

    pub fn slot_picker_next(&mut self) {
        if let Some(picker) = &mut self.slot_picker {
            picker.next();
        }
    }

    pub fn slot_picker_prev(&mut self) {
        if let Some(picker) = &mut self.slot_picker {
            picker.prev();
        }
    }

    pub fn selected_slot_item_id(&self) -> Option<i64> {
        self.slot_picker.as_ref()?.selected_item_id()
    }

    /// Description text for whatever is currently highlighted, for the screen's
    /// description box. Resolves through an open slot picker first (the popup is
    /// about a candidate item, not the empty slot behind it), then falls back to
    /// the focused pane's selection. Empty slots report `"<slot>: (empty)"`;
    /// nothing highlighted yields an empty string.
    pub fn selected_description(&self) -> String {
        if let Some(picker) = &self.slot_picker {
            return match picker.items.get(picker.selected_index) {
                Some(item) => item.description.clone(),
                None => "(no eligible items)".to_string(),
            };
        }
        match self.focus {
            InventoryFocus::Equipment => match self.slots.get(self.selected_slot_index) {
                Some(slot) => match &slot.equipped {
                    Some(item) => item.description.clone(),
                    None => format!("{}: (empty)", slot.slot_name),
                },
                None => String::new(),
            },
            InventoryFocus::Bag => match self.bag.get(self.selected_bag_index) {
                Some(item) => item.description.clone(),
                None => String::new(),
            },
        }
    }

    fn dialog_target(&self) -> Option<(i64, String, Vec<ItemAction>)> {
        match self.focus {
            InventoryFocus::Equipment => {
                let item = self
                    .slots
                    .get(self.selected_slot_index)?
                    .equipped
                    .as_ref()?;
                Some((
                    item.item_id,
                    item.name.clone(),
                    vec![ItemAction::Swap, ItemAction::Unequip, ItemAction::Drop],
                ))
            }
            InventoryFocus::Bag => {
                let item = self.bag.get(self.selected_bag_index)?;
                let mut actions = Vec::new();
                if item.usable {
                    actions.push(ItemAction::Use);
                }
                if item.equippable {
                    actions.push(ItemAction::Equip);
                }
                actions.push(ItemAction::Drop);
                Some((item.item_id, item.name.clone(), actions))
            }
        }
    }

    pub fn close_item_dialog(&mut self) {
        self.dialog = None;
    }

    pub fn item_dialog_next(&mut self) {
        if let Some(dialog) = &mut self.dialog {
            let len = dialog.actions.len();
            if len > 0 {
                dialog.selected_index = (dialog.selected_index + 1) % len;
            }
        }
    }

    pub fn item_dialog_prev(&mut self) {
        if let Some(dialog) = &mut self.dialog {
            let len = dialog.actions.len();
            if len > 0 {
                dialog.selected_index = (dialog.selected_index + len - 1) % len;
            }
        }
    }

    pub fn selected_action(&self) -> Option<ItemAction> {
        let dialog = self.dialog.as_ref()?;
        dialog.actions.get(dialog.selected_index).copied()
    }
}

#[cfg(test)]
mod tests;
