use crate::network::event::{InventoryItemInfo, InventorySlotInfo};

#[derive(Debug, Clone, PartialEq)]
pub enum InventoryFocus {
    Equipment,
    Bag,
}

#[derive(Debug, Clone)]
pub struct InventoryState {
    pub slots: Vec<InventorySlotInfo>,
    pub bag: Vec<InventoryItemInfo>,
    pub bag_size: usize,
    pub focus: InventoryFocus,
    pub selected_slot_index: usize,
    pub selected_bag_index: usize,
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
        InventoryItemInfo {
            item_id: 1,
            name: name.to_string(),
            item_type: "misc".to_string(),
            description: String::new(),
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
}
