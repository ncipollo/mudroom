use super::super::*;

pub(super) fn slot(name: &str) -> InventorySlotInfo {
    slot_accepting(name, &[])
}

pub(super) fn slot_accepting(name: &str, item_types: &[&str]) -> InventorySlotInfo {
    InventorySlotInfo {
        slot_name: name.to_string(),
        item_types: item_types.iter().map(|t| t.to_string()).collect(),
        equipped: None,
    }
}

pub(super) fn equipped_slot(
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

pub(super) fn item(name: &str) -> InventoryItemInfo {
    bag_item(name, false, false)
}

pub(super) fn bag_item(name: &str, usable: bool, equippable: bool) -> InventoryItemInfo {
    InventoryItemInfo {
        item_id: 1,
        name: name.to_string(),
        item_type: "misc".to_string(),
        description: String::new(),
        usable,
        equippable,
    }
}

pub(super) fn described_item(name: &str, description: &str) -> InventoryItemInfo {
    InventoryItemInfo {
        item_id: 1,
        name: name.to_string(),
        item_type: "misc".to_string(),
        description: description.to_string(),
        usable: false,
        equippable: false,
    }
}

pub(super) fn typed_item(item_id: i64, name: &str, item_type: &str) -> InventoryItemInfo {
    InventoryItemInfo {
        item_id,
        name: name.to_string(),
        item_type: item_type.to_string(),
        description: String::new(),
        usable: false,
        equippable: true,
    }
}
