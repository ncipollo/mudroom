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
    Unequip,
    Drop,
}

impl ItemAction {
    pub fn label(self) -> &'static str {
        match self {
            ItemAction::Use => "Use",
            ItemAction::Equip => "Equip",
            ItemAction::Unequip => "Unequip",
            ItemAction::Drop => "Drop",
        }
    }
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

    /// Opens the item-action dialog for the currently focused+selected item. A no-op when the
    /// equipment pane is focused on an empty slot, or the bag is empty.
    pub fn open_item_dialog(&mut self) {
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
                    vec![ItemAction::Unequip, ItemAction::Drop],
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
        InventorySlotInfo {
            slot_name: name.to_string(),
            equipped: None,
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
    fn open_item_dialog_is_noop_on_empty_equipment_slot() {
        let mut state = InventoryState::new(vec![slot("weapon")], vec![], 0);
        state.open_item_dialog();
        assert!(state.dialog.is_none());
    }

    #[test]
    fn open_item_dialog_on_equipped_item_offers_unequip_and_drop() {
        let mut state = InventoryState::new(
            vec![InventorySlotInfo {
                slot_name: "weapon".to_string(),
                equipped: Some(item("Sword")),
            }],
            vec![],
            0,
        );
        state.open_item_dialog();
        let dialog = state.dialog.unwrap();
        assert_eq!(dialog.item_name, "Sword");
        assert_eq!(dialog.actions, vec![ItemAction::Unequip, ItemAction::Drop]);
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
