use std::collections::HashMap;
use std::sync::Arc;

use crate::game::GameState;
use crate::game::component::{Inventory, Item, ItemDefinition, ItemUseType};
use crate::game::config::inventory_config::{EquipmentSlotConfig, InventoryDefinition};
use crate::game::messaging::{self, InventoryItemInfo, InventoryOpenedMessage, InventorySlotInfo};
use crate::game::player::Player;

pub async fn process(game_state: &Arc<GameState>, player: &Player) {
    let Some(inventory) = character_inventory(game_state, player).await else {
        return;
    };

    let definitions = game_state.item_definitions.read().await;
    let payload = build_payload(game_state, &inventory, &definitions);
    messaging::inventory_opened(&game_state.message_tx, player.id, payload);
}

async fn character_inventory(game_state: &Arc<GameState>, player: &Player) -> Option<Inventory> {
    let characters = game_state.active_characters.read().await;
    characters
        .get(&player.entity_id)
        .map(|character| character.inventory.clone())
}

fn build_payload(
    game_state: &Arc<GameState>,
    inventory: &Inventory,
    definitions: &HashMap<String, ItemDefinition>,
) -> InventoryOpenedMessage {
    let resolved_def = game_state
        .inventory_config
        .resolve(&inventory.inventory_type);
    let equipment_slots: &[EquipmentSlotConfig] = resolved_def
        .map(|def| def.equipment_slots.as_slice())
        .unwrap_or(&[]);
    let slots = build_slots(resolved_def, inventory, definitions, equipment_slots);
    let bag = inventory
        .bag
        .iter()
        .filter_map(|item| resolve_item(item, definitions, equipment_slots))
        .collect();
    let bag_size = resolved_def
        .map(|def| def.bag_size)
        .unwrap_or(inventory.bag.len());

    InventoryOpenedMessage {
        slots,
        bag,
        bag_size,
    }
}

fn build_slots(
    definition: Option<&InventoryDefinition>,
    inventory: &Inventory,
    definitions: &HashMap<String, ItemDefinition>,
    equipment_slots: &[EquipmentSlotConfig],
) -> Vec<InventorySlotInfo> {
    let Some(definition) = definition else {
        return Vec::new();
    };
    definition
        .equipment_slots
        .iter()
        .map(|slot| InventorySlotInfo {
            slot_name: slot.name.clone(),
            item_types: slot.item_types.clone(),
            equipped: inventory
                .equipment
                .get(&slot.name)
                .and_then(|item| resolve_item(item, definitions, equipment_slots)),
        })
        .collect()
}

fn resolve_item(
    item: &Item,
    definitions: &HashMap<String, ItemDefinition>,
    equipment_slots: &[EquipmentSlotConfig],
) -> Option<InventoryItemInfo> {
    let definition = definitions.get(&item.item_definition_id)?;
    let equippable = equipment_slots
        .iter()
        .any(|slot| slot.item_types.iter().any(|t| t == &definition.item_type));
    Some(InventoryItemInfo {
        item_id: item.id,
        name: definition.name.clone(),
        item_type: definition.item_type.clone(),
        description: definition.description.text.clone().unwrap_or_default(),
        usable: definition.use_type == ItemUseType::Used,
        equippable,
    })
}
