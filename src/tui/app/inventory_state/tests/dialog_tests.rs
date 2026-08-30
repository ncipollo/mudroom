use super::super::*;
use super::support::{bag_item, equipped_slot, item, slot, slot_accepting, typed_item};

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
