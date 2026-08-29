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
mod tests {
    use super::*;

    fn slot(name: &str) -> InventorySlotInfo {
        slot_accepting(name, &[])
    }

    fn slot_accepting(name: &str, item_types: &[&str]) -> InventorySlotInfo {
        InventorySlotInfo {
            slot_name: name.to_string(),
            item_types: item_types.iter().map(|t| t.to_string()).collect(),
            equipped: None,
        }
    }

    fn equipped_slot(
        name: &str,
        item_types: &[&str],
        equipped: InventoryItemInfo,
    ) -> InventorySlotInfo {
        InventorySlotInfo {
            slot_name: name.to_string(),
            item_types: item_types.iter().map(|t| t.to_string()).collect(),
            equipped: Some(equipped),
        }
    }

    fn item(name: &str) -> InventoryItemInfo {
        bag_item(name, false, false)
    }

    fn bag_item(name: &str, usable: bool, equippable: bool) -> InventoryItemInfo {
        InventoryItemInfo {
            item_id: 1,
            name: name.to_string(),
            item_type: "misc".to_string(),
            description: String::new(),
            usable,
            equippable,
        }
    }

    fn typed_item(item_id: i64, name: &str, item_type: &str) -> InventoryItemInfo {
        InventoryItemInfo {
            item_id,
            name: name.to_string(),
            item_type: item_type.to_string(),
            description: String::new(),
            usable: false,
            equippable: true,
        }
    }

    #[test]
    fn toggle_focus_flips_between_equipment_and_bag() {
        let mut state = InventoryState::new(vec![], vec![], 0);
        assert_eq!(state.focus, InventoryFocus::Equipment);
        state.toggle_focus();
        assert_eq!(state.focus, InventoryFocus::Bag);
        state.toggle_focus();
        assert_eq!(state.focus, InventoryFocus::Equipment);
    }

    #[test]
    fn select_next_wraps_within_equipment_slots() {
        let mut state = InventoryState::new(vec![slot("weapon"), slot("armor")], vec![], 0);
        state.select_next();
        assert_eq!(state.selected_slot_index, 1);
        state.select_next();
        assert_eq!(state.selected_slot_index, 0);
    }

    #[test]
    fn select_prev_wraps_within_bag_items() {
        let mut state = InventoryState::new(vec![], vec![item("Potion"), item("Sword")], 10);
        state.toggle_focus();
        state.select_prev();
        assert_eq!(state.selected_bag_index, 1);
        state.select_prev();
        assert_eq!(state.selected_bag_index, 0);
    }

    #[test]
    fn select_next_is_noop_when_pane_is_empty() {
        let mut state = InventoryState::new(vec![], vec![], 0);
        state.select_next();
        assert_eq!(state.selected_slot_index, 0);
        state.toggle_focus();
        state.select_next();
        assert_eq!(state.selected_bag_index, 0);
    }

    #[test]
    fn open_item_dialog_is_noop_on_empty_equipment_slot_with_no_matching_items() {
        let mut state = InventoryState::new(vec![slot("weapon")], vec![], 0);
        state.open_item_dialog();
        assert!(state.dialog.is_none());
        assert!(state.slot_picker.is_none());
    }

    #[test]
    fn open_item_dialog_on_equipped_item_offers_swap_unequip_and_drop() {
        let mut state = InventoryState::new(
            vec![equipped_slot("weapon", &["weapon"], item("Sword"))],
            vec![],
            0,
        );
        state.open_item_dialog();
        let dialog = state.dialog.unwrap();
        assert_eq!(dialog.item_name, "Sword");
        assert_eq!(
            dialog.actions,
            vec![ItemAction::Swap, ItemAction::Unequip, ItemAction::Drop]
        );
    }

    #[test]
    fn open_item_dialog_on_empty_slot_opens_picker_with_only_matching_items() {
        let mut state = InventoryState::new(
            vec![slot_accepting("weapon", &["weapon"])],
            vec![
                typed_item(1, "Sword", "weapon"),
                typed_item(2, "Potion", "consumable"),
            ],
            10,
        );
        state.open_item_dialog();
        assert!(state.dialog.is_none());
        let picker = state.slot_picker.expect("picker should open");
        assert_eq!(picker.slot_name, "weapon");
        assert_eq!(picker.items.len(), 1);
        assert_eq!(picker.items[0].name, "Sword");
    }

    #[test]
    fn open_slot_picker_is_noop_when_no_bag_item_fits() {
        let mut state = InventoryState::new(
            vec![slot_accepting("weapon", &["weapon"])],
            vec![typed_item(1, "Potion", "consumable")],
            10,
        );
        state.open_slot_picker();
        assert!(state.slot_picker.is_none());
    }

    #[test]
    fn open_slot_picker_from_occupied_slot_replaces_action_dialog() {
        let mut state = InventoryState::new(
            vec![equipped_slot(
                "weapon",
                &["weapon"],
                typed_item(1, "Old Sword", "weapon"),
            )],
            vec![typed_item(2, "New Sword", "weapon")],
            10,
        );
        state.open_item_dialog();
        assert!(state.dialog.is_some());
        state.open_slot_picker();
        assert!(state.dialog.is_none());
        let picker = state.slot_picker.unwrap();
        assert_eq!(picker.slot_name, "weapon");
        assert_eq!(picker.items.len(), 1);
        assert_eq!(picker.items[0].name, "New Sword");
    }

    #[test]
    fn slot_picker_next_and_prev_wrap() {
        let mut state = InventoryState::new(
            vec![slot_accepting("weapon", &["weapon"])],
            vec![
                typed_item(1, "Sword", "weapon"),
                typed_item(2, "Axe", "weapon"),
            ],
            10,
        );
        state.open_slot_picker();
        assert_eq!(state.selected_slot_item_id(), Some(1));
        state.slot_picker_next();
        assert_eq!(state.selected_slot_item_id(), Some(2));
        state.slot_picker_next();
        assert_eq!(state.selected_slot_item_id(), Some(1));
        state.slot_picker_prev();
        assert_eq!(state.selected_slot_item_id(), Some(2));
    }

    #[test]
    fn close_slot_picker_clears_picker() {
        let mut state = InventoryState::new(
            vec![slot_accepting("weapon", &["weapon"])],
            vec![typed_item(1, "Sword", "weapon")],
            10,
        );
        state.open_slot_picker();
        assert!(state.slot_picker.is_some());
        state.close_slot_picker();
        assert!(state.slot_picker.is_none());
    }

    #[test]
    fn open_item_dialog_on_usable_equippable_bag_item_offers_all_actions() {
        let mut state = InventoryState::new(vec![], vec![bag_item("Sword", true, true)], 10);
        state.toggle_focus();
        state.open_item_dialog();
        let dialog = state.dialog.unwrap();
        assert_eq!(
            dialog.actions,
            vec![ItemAction::Use, ItemAction::Equip, ItemAction::Drop]
        );
    }

    #[test]
    fn open_item_dialog_on_plain_bag_item_only_offers_drop() {
        let mut state = InventoryState::new(vec![], vec![bag_item("Rock", false, false)], 10);
        state.toggle_focus();
        state.open_item_dialog();
        let dialog = state.dialog.unwrap();
        assert_eq!(dialog.actions, vec![ItemAction::Drop]);
    }

    #[test]
    fn item_dialog_next_and_prev_wrap() {
        let mut state = InventoryState::new(vec![], vec![bag_item("Sword", true, true)], 10);
        state.toggle_focus();
        state.open_item_dialog();
        state.item_dialog_next();
        assert_eq!(state.selected_action(), Some(ItemAction::Equip));
        state.item_dialog_next();
        assert_eq!(state.selected_action(), Some(ItemAction::Drop));
        state.item_dialog_next();
        assert_eq!(state.selected_action(), Some(ItemAction::Use));
        state.item_dialog_prev();
        assert_eq!(state.selected_action(), Some(ItemAction::Drop));
    }

    #[test]
    fn close_item_dialog_clears_dialog() {
        let mut state = InventoryState::new(vec![], vec![bag_item("Sword", true, true)], 10);
        state.toggle_focus();
        state.open_item_dialog();
        state.close_item_dialog();
        assert!(state.dialog.is_none());
    }
}
