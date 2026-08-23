use std::collections::HashMap;
use std::sync::Arc;

use tracing;

use crate::game::component::{
    Attribute, Effect, EffectType, Item, ItemDefinition, ItemUseType, Modifier, Operator,
    TriggerInfo, UseEffect,
};
use crate::game::interaction::inventory;
use crate::game::player::Player;
use crate::game::{GameState, messaging};
use crate::persistence::Database;
use crate::persistence::PersistenceError;
use crate::persistence::{character_effect_repo, character_repo, inventory_repo};

struct UseSnapshot {
    character_id: i64,
    item: Item,
    definition: ItemDefinition,
    attributes: HashMap<String, Attribute>,
}

pub async fn process(game_state: &Arc<GameState>, db: &Database, player: &Player, item_id: i64) {
    let Some(snapshot) = character_snapshot(game_state, player, item_id).await else {
        messaging::message(
            &game_state.message_tx,
            player.id,
            "You don't have that item.",
        );
        return;
    };

    if snapshot.definition.use_type != ItemUseType::Used {
        messaging::message(
            &game_state.message_tx,
            player.id,
            format!("You can't use the {}.", snapshot.definition.name),
        );
        return;
    }

    let mut attributes = snapshot.attributes.clone();
    let over_time_effects = apply_use_effects(&snapshot.definition.use_effects, &mut attributes);

    persist_use(db, &snapshot, &attributes, &over_time_effects).await;
    apply_in_memory(game_state, player, &snapshot, attributes, over_time_effects).await;
    sync_player_stats(game_state, player).await;

    messaging::message(
        &game_state.message_tx,
        player.id,
        format!("You use the {}.", snapshot.definition.name),
    );
    inventory::process(game_state, player).await;
}

async fn character_snapshot(
    game_state: &Arc<GameState>,
    player: &Player,
    item_id: i64,
) -> Option<UseSnapshot> {
    let characters = game_state.active_characters.read().await;
    let character = characters.get(&player.entity_id)?;
    let item = character
        .inventory
        .bag
        .iter()
        .find(|i| i.id == item_id)?
        .clone();
    let attributes = character.attributes.clone();
    let character_id = character.id;
    drop(characters);

    let definitions = game_state.item_definitions.read().await;
    let definition = definitions.get(&item.item_definition_id)?.clone();

    Some(UseSnapshot {
        character_id,
        item,
        definition,
        attributes,
    })
}

/// Applies stat boosts and `Once`-trigger effects immediately, returning `OverTime` effects to
/// be recorded (but not yet ticked — the world effect engine is a separate, currently-stubbed
/// concern; see `game_loop/effects.rs`).
fn apply_use_effects(
    effects: &[UseEffect],
    attributes: &mut HashMap<String, Attribute>,
) -> Vec<Effect> {
    let mut over_time = Vec::new();
    for effect in effects {
        match effect {
            UseEffect::StatBoost(modifier) => apply_modifier(attributes, modifier),
            UseEffect::ApplyEffect(effect) => apply_use_effect(effect, attributes, &mut over_time),
        }
    }
    over_time
}

fn apply_modifier(attributes: &mut HashMap<String, Attribute>, modifier: &Modifier) {
    let Some(attr) = attributes.get_mut(&modifier.attribute_id) else {
        return;
    };
    let new_value = match modifier.operator {
        Operator::Add => attr.current_value + modifier.amount,
        Operator::Subtract => attr.current_value - modifier.amount,
        Operator::Multiply => attr.current_value * modifier.amount,
        Operator::Divide if modifier.amount != 0 => attr.current_value / modifier.amount,
        Operator::Divide => attr.current_value,
    };
    attr.current_value = new_value.clamp(attr.min_value, attr.max_value);
}

fn apply_use_effect(
    effect: &Effect,
    attributes: &mut HashMap<String, Attribute>,
    over_time: &mut Vec<Effect>,
) {
    match effect.trigger_info {
        TriggerInfo::Once => apply_once_effect(effect, attributes),
        TriggerInfo::OverTime { .. } => over_time.push(effect.clone()),
    }
}

fn apply_once_effect(effect: &Effect, attributes: &mut HashMap<String, Attribute>) {
    let EffectType::AttributeUpdate {
        attribute_id,
        value,
    } = &effect.effect_type
    else {
        return;
    };
    let Some(attr) = attributes.get_mut(attribute_id) else {
        return;
    };
    attr.current_value = (attr.current_value + value).clamp(attr.min_value, attr.max_value);
}

async fn persist_use(
    db: &Database,
    snapshot: &UseSnapshot,
    attributes: &HashMap<String, Attribute>,
    over_time_effects: &[Effect],
) {
    log_on_error(
        character_repo::update_attributes(db.pool(), snapshot.character_id, attributes).await,
        "update attributes after item use",
    );
    for effect in over_time_effects {
        persist_over_time_effect(db, snapshot.character_id, effect).await;
    }
    log_on_error(
        inventory_repo::remove_item(db.pool(), snapshot.item.id).await,
        "remove used item",
    );
}

async fn persist_over_time_effect(db: &Database, character_id: i64, effect: &Effect) {
    log_on_error(
        character_effect_repo::insert(db.pool(), character_id, effect).await,
        "persist use-effect",
    );
}

fn log_on_error<T>(result: Result<T, PersistenceError>, action: &str) -> Option<T> {
    result
        .inspect_err(|e| tracing::error!("Failed to {action}: {e}"))
        .ok()
}

async fn apply_in_memory(
    game_state: &Arc<GameState>,
    player: &Player,
    snapshot: &UseSnapshot,
    attributes: HashMap<String, Attribute>,
    over_time_effects: Vec<Effect>,
) {
    let mut characters = game_state.active_characters.write().await;
    if let Some(character) = characters.get_mut(&player.entity_id) {
        character.attributes = attributes;
        character.active_effects.extend(over_time_effects);
        character.inventory.bag.retain(|i| i.id != snapshot.item.id);
    }
}

/// Pushes the character's current HP/MP to the client after a use-item effect is applied, so the
/// world-view status bar reflects the change (e.g. a heal from a consumable).
async fn sync_player_stats(game_state: &Arc<GameState>, player: &Player) {
    let characters = game_state.active_characters.read().await;
    let Some(character) = characters.get(&player.entity_id) else {
        return;
    };
    let hp_attr_id = messaging::hp_attribute_id(&game_state.attribute_config);
    let mp_attr_id = messaging::mp_attribute_id(&game_state.attribute_config);
    let (hp_current, hp_max) = character.attribute_range(&hp_attr_id);
    let (mp_current, mp_max) = character.attribute_range(&mp_attr_id);
    messaging::player_stats_updated(
        &game_state.message_tx,
        player.id,
        hp_current,
        hp_max,
        mp_current,
        mp_max,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::{Description, EffectDescription, EquippedBonuses, Location};
    use crate::game::entity::character::{Character, CharacterType};
    use crate::game::messaging::Message;
    use crate::game::{Dungeon, Room, World};
    use crate::persistence::{
        character_repo, dungeon_repo, inventory_repo, item_repo, room_repo, world_repo,
    };

    fn test_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    async fn setup(db: &Database) -> i64 {
        world_repo::insert(db.pool(), &World::new("w1".to_string()))
            .await
            .unwrap();
        dungeon_repo::insert(db.pool(), &Dungeon::new("d1".to_string()), "w1")
            .await
            .unwrap();
        let room = Room::new("r1".to_string(), Description::new(None));
        room_repo::insert(db.pool(), &room, "d1").await.unwrap();

        let character = Character::new(0, CharacterType::Player, test_location());
        character_repo::insert(db.pool(), &character).await.unwrap()
    }

    fn test_player(entity_id: i64) -> Player {
        Player {
            id: 1,
            client_id: "client".to_string(),
            name: "Hero".to_string(),
            entity_id,
        }
    }

    fn tonic_definition(use_effects: Vec<UseEffect>) -> ItemDefinition {
        ItemDefinition {
            id: "tonic".to_string(),
            name: "Tonic".to_string(),
            description: Description::default(),
            use_type: ItemUseType::Used,
            item_type: "consumable".to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects,
        }
    }

    async fn seed_character(
        game_state: &Arc<GameState>,
        db: &Database,
        character_id: i64,
        definition: ItemDefinition,
    ) -> i64 {
        item_repo::upsert_definition(db.pool(), &definition)
            .await
            .unwrap();
        let item_id = inventory_repo::add_bag_item(db.pool(), character_id, &definition.id)
            .await
            .unwrap();
        game_state
            .item_definitions
            .write()
            .await
            .insert(definition.id.clone(), definition);

        let mut character = Character::new(character_id, CharacterType::Player, test_location());
        character.attributes.insert(
            "hp".to_string(),
            Attribute::new("hp".to_string(), 0, 100, 50),
        );
        character.attributes.insert(
            "strength".to_string(),
            Attribute::new("strength".to_string(), 0, 20, 5),
        );
        character.inventory.bag.push(Item {
            id: item_id,
            item_definition_id: "tonic".to_string(),
        });
        game_state
            .active_characters
            .write()
            .await
            .insert(character_id, character);

        item_id
    }

    #[tokio::test]
    async fn use_item_applies_once_effect_and_consumes_item() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        let definition = tonic_definition(vec![UseEffect::ApplyEffect(Effect {
            name: "heal".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: 20,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
            scope: Default::default(),
        })]);
        let item_id = seed_character(&game_state, &db, character_id, definition).await;
        let player = test_player(character_id);

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, item_id).await;

        let characters = game_state.active_characters.read().await;
        let character = &characters[&character_id];
        assert_eq!(character.attributes["hp"].current_value, 70);
        assert!(character.inventory.bag.is_empty());
        drop(characters);

        let bag = inventory_repo::find_bag_by_character(db.pool(), character_id)
            .await
            .unwrap();
        assert!(bag.is_empty());

        let stats_msg = rx.recv().await.unwrap();
        assert!(matches!(
            stats_msg.message,
            Message::PlayerStatsUpdated {
                hp_current: 70,
                hp_max: 100,
                ..
            }
        ));

        let msg = rx.recv().await.unwrap();
        match msg.message {
            Message::Complete { content, .. } => assert_eq!(content, "You use the Tonic."),
            other => panic!("expected Complete message, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn use_item_applies_stat_boost() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        let definition = tonic_definition(vec![UseEffect::StatBoost(Modifier {
            attribute_id: "strength".to_string(),
            operator: Operator::Add,
            amount: 3,
        })]);
        let item_id = seed_character(&game_state, &db, character_id, definition).await;
        let player = test_player(character_id);

        process(&game_state, &db, &player, item_id).await;

        let characters = game_state.active_characters.read().await;
        assert_eq!(
            characters[&character_id].attributes["strength"].current_value,
            8
        );
    }

    #[tokio::test]
    async fn use_item_records_over_time_effect_without_applying_it() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        let definition = tonic_definition(vec![UseEffect::ApplyEffect(Effect {
            name: "regen".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: 5,
            },
            trigger_info: TriggerInfo::OverTime {
                start: 0,
                end: None,
                rate: 1,
            },
            description: EffectDescription::default(),
            scope: Default::default(),
        })]);
        let item_id = seed_character(&game_state, &db, character_id, definition).await;
        let player = test_player(character_id);

        process(&game_state, &db, &player, item_id).await;

        let characters = game_state.active_characters.read().await;
        let character = &characters[&character_id];
        assert_eq!(character.attributes["hp"].current_value, 50);
        assert_eq!(character.active_effects.len(), 1);
    }

    #[tokio::test]
    async fn use_item_rejects_passive_item() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        let mut definition = tonic_definition(vec![]);
        definition.use_type = ItemUseType::Passive;
        let item_id = seed_character(&game_state, &db, character_id, definition).await;
        let player = test_player(character_id);

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, item_id).await;

        let characters = game_state.active_characters.read().await;
        assert_eq!(characters[&character_id].inventory.bag.len(), 1);
        drop(characters);

        let msg = rx.recv().await.unwrap();
        match msg.message {
            Message::Complete { content, .. } => assert_eq!(content, "You can't use the Tonic."),
            other => panic!("expected Complete message, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn use_item_rejects_missing_item() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        game_state.active_characters.write().await.insert(
            character_id,
            Character::new(character_id, CharacterType::Player, test_location()),
        );
        let player = test_player(character_id);

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, 999).await;

        let msg = rx.recv().await.unwrap();
        match msg.message {
            Message::Complete { content, .. } => assert_eq!(content, "You don't have that item."),
            other => panic!("expected Complete message, got {other:?}"),
        }
    }
}
