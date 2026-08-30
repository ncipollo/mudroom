use super::super::*;
use super::support::{described_item, equipped_slot, item, slot, slot_accepting, typed_item};

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
fn selected_description_returns_highlighted_bag_item_description() {
    let mut state = InventoryState::new(
        vec![],
        vec![
            described_item("Potion", "Restores health."),
            described_item("Elixir", "Restores mana."),
        ],
        10,
    );
    state.toggle_focus();
    assert_eq!(state.selected_description(), "Restores health.");
    state.select_next();
    assert_eq!(state.selected_description(), "Restores mana.");
}

#[test]
fn selected_description_returns_equipped_item_description() {
    let state = InventoryState::new(
        vec![equipped_slot(
            "weapon",
            &["weapon"],
            described_item("Sword", "A sharp blade."),
        )],
        vec![],
        0,
    );
    assert_eq!(state.selected_description(), "A sharp blade.");
}

#[test]
fn selected_description_indicates_empty_equipment_slot() {
    let state = InventoryState::new(vec![slot("weapon")], vec![], 0);
    assert_eq!(state.selected_description(), "weapon: (empty)");
}

#[test]
fn selected_description_is_empty_for_empty_bag() {
    let mut state = InventoryState::new(vec![], vec![], 0);
    state.toggle_focus();
    assert_eq!(state.selected_description(), "");
}

#[test]
fn selected_description_follows_open_slot_picker_selection() {
    let mut state = InventoryState::new(
        vec![slot_accepting("weapon", &["weapon"])],
        vec![
            InventoryItemInfo {
                description: "The old one.".to_string(),
                ..typed_item(1, "Sword", "weapon")
            },
            InventoryItemInfo {
                description: "The new one.".to_string(),
                ..typed_item(2, "Axe", "weapon")
            },
        ],
        10,
    );
    state.open_slot_picker();
    assert_eq!(state.selected_description(), "The old one.");
    state.slot_picker_next();
    assert_eq!(state.selected_description(), "The new one.");
}
