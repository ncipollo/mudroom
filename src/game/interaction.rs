pub mod conversation;
pub mod drop;
pub mod equip;
pub mod help;
pub mod inventory;
pub mod lifecycle;
pub mod look;
pub mod movement;
pub mod room_threats;
pub mod take;
pub mod use_item;

use std::collections::HashMap;
use std::sync::Arc;

use tracing;

use crate::game::component::interaction::Movement;
use crate::game::component::{Ability, ItemDefinition};
use crate::game::engagement::TurnAction;
use crate::game::engagement::battle;
use crate::game::entity::character::Character;
use crate::game::messaging;
use crate::game::player::Player;
use crate::game::{GameState, Interaction};
use crate::persistence::Database;

pub async fn process(game_state: &Arc<GameState>, db: &Database, tick: u64) {
    tracing::debug!("Processing interactions tick={tick}");

    let players: Vec<(String, Player)> = game_state
        .active_players
        .read()
        .await
        .iter()
        .map(|(client_id, player)| (client_id.clone(), player.clone()))
        .collect();

    for (client_id, player) in players {
        process_player(game_state, db, &client_id, &player).await;
    }
}

async fn process_player(
    game_state: &Arc<GameState>,
    db: &Database,
    client_id: &str,
    player: &Player,
) {
    let interactions = game_state.mailboxes.drain(player.entity_id).await;
    for interaction in interactions {
        dispatch_interaction(game_state, db, client_id, player, interaction).await;
    }
}

async fn dispatch_interaction(
    game_state: &Arc<GameState>,
    db: &Database,
    client_id: &str,
    player: &Player,
    interaction: Interaction,
) {
    match interaction {
        Interaction::Look => look::process(game_state, db, player, false).await,
        Interaction::EnterRoom => look::process(game_state, db, player, true).await,
        Interaction::LookAt { target } => {
            look::process_at(game_state, db, player, &target).await;
        }
        Interaction::Help => help::process(game_state, player).await,
        Interaction::Take { target } => {
            take::process(game_state, db, player, &target).await;
        }
        Interaction::Movement(m) => dispatch_movement(game_state, db, player, m).await,
        Interaction::EngagementAction(action) => {
            dispatch_engagement_action(game_state, player, action).await;
        }
        conv @ (Interaction::StartConversation { .. } | Interaction::EndConversation) => {
            dispatch_conversation(game_state, player, conv).await;
        }
        Interaction::JoinBattle { .. } => {
            dispatch_join_battle(game_state, player).await;
        }
        Interaction::LeaveBattle { .. } => {
            dispatch_leave_battle(game_state, player).await;
        }
        Interaction::CheckRoomThreats { room_id } => {
            room_threats::check_room_hostility(game_state, player, &room_id).await;
        }
        item_action @ (Interaction::OpenInventory
        | Interaction::UseItem { .. }
        | Interaction::EquipItem { .. }
        | Interaction::UnequipItem { .. }
        | Interaction::DropItem { .. }) => {
            dispatch_item_action(game_state, db, player, item_action).await;
        }
        Interaction::PlayerDisconnected {
            client_id: disconnected_client_id,
            epoch,
        } => {
            dispatch_player_disconnected(
                game_state,
                client_id,
                player,
                disconnected_client_id,
                epoch,
            )
            .await;
        }
    }
}

/// Tears down a truly disconnected player. Skips teardown if the character's activation epoch has
/// advanced past the one this `PlayerDisconnected` was queued under — the player already
/// reactivated. Epoch-based rather than client_id-based because `ClientSession` reuses the same
/// id across reconnects, so it's correct regardless of how late the disconnect lands.
async fn dispatch_player_disconnected(
    game_state: &Arc<GameState>,
    client_id: &str,
    player: &Player,
    disconnected_client_id: String,
    epoch: u64,
) {
    let current_epoch = game_state.current_activation_epoch(player.entity_id).await;
    if epoch != current_epoch {
        log_stale_disconnect(
            player,
            client_id,
            disconnected_client_id,
            epoch,
            current_epoch,
        );
        return;
    }
    tracing::info!(
        entity_id = player.entity_id,
        disconnected_client_id,
        client_id,
        epoch,
        "tearing down disconnected player"
    );
    lifecycle::player_disconnected(game_state, player).await;
}

fn log_stale_disconnect(
    player: &Player,
    client_id: &str,
    disconnected_client_id: String,
    disconnect_epoch: u64,
    current_epoch: u64,
) {
    tracing::info!(
        entity_id = player.entity_id,
        disconnected_client_id,
        client_id,
        disconnect_epoch,
        current_epoch,
        "ignoring stale disconnect superseded by a later activation"
    );
}

async fn dispatch_item_action(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    interaction: Interaction,
) {
    match interaction {
        Interaction::OpenInventory => {
            inventory::process(game_state, player).await;
        }
        Interaction::UseItem { item_id } => {
            use_item::process(game_state, db, player, item_id).await;
        }
        Interaction::EquipItem { item_id } => {
            equip::process(game_state, db, player, item_id).await;
        }
        Interaction::UnequipItem { item_id } => {
            equip::unequip(game_state, db, player, item_id).await;
        }
        Interaction::DropItem { item_id } => {
            drop::process(game_state, db, player, item_id).await;
        }
        _ => {}
    }
}

async fn dispatch_movement(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    movement: Movement,
) {
    match movement {
        Movement::TryDirection(direction) => {
            movement::process(game_state, db, player, direction).await;
        }
        Movement::Warp(_) => {}
    }
}

async fn dispatch_engagement_action(
    game_state: &Arc<GameState>,
    player: &Player,
    action: TurnAction,
) {
    match action {
        TurnAction::QueueAbility {
            ability_id,
            target_id,
        } => {
            dispatch_queue_ability(game_state, player, &ability_id, target_id).await;
        }
        TurnAction::SkipPhase => {
            game_state
                .engagements
                .battles
                .skip_phase(player.entity_id)
                .await;
        }
        other => {
            let accepted = game_state
                .engagements
                .conversations
                .submit_action_for_entity(player.entity_id, other)
                .await;
            tracing::debug!(
                entity_id = player.entity_id,
                accepted,
                "engagement action submitted"
            );
        }
    }
}

async fn dispatch_queue_ability(
    game_state: &Arc<GameState>,
    player: &Player,
    ability_id: &str,
    target_id: i64,
) {
    let (ability_opt, attrs) = {
        let entities = game_state.active_characters.read().await;
        let Some(character) = entities.get(&player.entity_id) else {
            return;
        };
        let item_definitions = game_state.item_definitions.read().await;
        let abilities = game_state.abilities.read().await;
        let ability = queueable_ability(character, &item_definitions, &abilities, ability_id);
        (ability, character.attributes.clone())
    };
    let Some(ability) = ability_opt else {
        return;
    };
    let accepted = game_state
        .engagements
        .battles
        .queue_ability(player.entity_id, ability, target_id, &attrs)
        .await;
    tracing::debug!(
        entity_id = player.entity_id,
        accepted,
        "battle ability queued"
    );
}

/// Resolves `ability_id` against everything the character can currently queue in battle —
/// innate abilities plus any granted by equipped items — not just `innate_abilities` alone, so
/// an ability granted by gear (e.g. a weapon's `equipped_bonuses.equipped`) can actually be
/// queued, not just displayed in the battle UI's ability list (which already used
/// `combined_abilities` via `battle::abilities::entity_battle_abilities`).
fn queueable_ability(
    character: &Character,
    item_definitions: &HashMap<String, ItemDefinition>,
    abilities: &HashMap<String, Ability>,
    ability_id: &str,
) -> Option<Ability> {
    character
        .combined_abilities(item_definitions, abilities)
        .into_iter()
        .find(|a| a.id == ability_id)
}

async fn dispatch_conversation(
    game_state: &Arc<GameState>,
    player: &Player,
    interaction: Interaction,
) {
    match interaction {
        Interaction::StartConversation { initial_message } => {
            conversation::process(game_state, player, initial_message).await;
        }
        Interaction::EndConversation => {
            conversation::end_player_conversation(game_state, player).await;
        }
        _ => {}
    }
}

async fn dispatch_join_battle(game_state: &Arc<GameState>, player: &Player) {
    let room_id = {
        let entities = game_state.active_characters.read().await;
        entities
            .get(&player.entity_id)
            .map(|e| e.location.room_id.clone())
    };
    let Some(room_id) = room_id else {
        return;
    };

    let Some(engagement_id) = game_state.engagements.battles.find_for_room(&room_id).await else {
        messaging::message(
            &game_state.message_tx,
            player.id,
            "There is no active battle here.",
        );
        return;
    };

    let faction = {
        let entities = game_state.active_characters.read().await;
        entities
            .get(&player.entity_id)
            .and_then(|e| e.factions.iter().next().cloned())
            .unwrap_or_else(|| "player".to_string())
    };

    battle::participants::add_entity(
        &game_state.engagements.battles,
        engagement_id,
        &faction,
        player.entity_id,
    )
    .await;

    let Some((factions, participants)) =
        battle::participants::snapshot(&game_state.engagements.battles, engagement_id).await
    else {
        return;
    };

    let max_turn_ticks = (game_state.mud_config.game_loop.max_engage_ms
        / game_state.mud_config.game_loop.tick_rate_ms)
        .max(1);

    let started_msg = room_threats::build_battle_started_message(
        game_state,
        player,
        engagement_id,
        &factions,
        &participants,
        max_turn_ticks,
    )
    .await;

    messaging::battle_started(&game_state.message_tx, player.id, started_msg);
    messaging::message(&game_state.message_tx, player.id, "You join the battle!");
}

async fn dispatch_leave_battle(game_state: &Arc<GameState>, player: &Player) {
    let Some((engagement_id, surviving)) =
        battle::participants::remove_entity(&game_state.engagements.battles, player.entity_id)
            .await
    else {
        return;
    };

    if surviving <= 1 {
        battle::end_battle(game_state, engagement_id, &[player.entity_id]).await;
    }

    messaging::battle_ended(&game_state.message_tx, player.id, engagement_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::Location;
    use crate::game::entity::character::{Character, CharacterType};
    use std::collections::HashMap;

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    fn test_player(client_id: &str) -> Player {
        Player {
            id: 1,
            client_id: client_id.to_string(),
            name: "Hero".to_string(),
            entity_id: 1,
        }
    }

    async fn battle_state(registered_client_id: &str) -> Arc<GameState> {
        let game_state = Arc::new(GameState::load(None).unwrap());
        {
            let mut entities = game_state.active_characters.write().await;
            entities.insert(1, Character::new(1, CharacterType::Player, test_location()));
            entities.insert(2, Character::new(2, CharacterType::Enemy, test_location()));
        }
        game_state.active_players.write().await.insert(
            registered_client_id.to_string(),
            test_player(registered_client_id),
        );
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);
        participants.insert("enemy".to_string(), vec![2]);
        game_state
            .engagements
            .add_battle(
                "r".to_string(),
                vec!["player".to_string(), "enemy".to_string()],
                participants,
            )
            .await;
        game_state
    }

    #[test]
    fn queueable_ability_resolves_equipment_granted_ability() {
        use crate::game::component::{
            AbilityRole, Description, EquippedBonuses, Item, ItemUseType,
        };
        use crate::game::engagement::EngagementType;
        use crate::game::entity::character::CharacterType;

        let mut character = Character::new(1, CharacterType::Player, test_location());
        character.inventory.equipment.insert(
            "weapon".to_string(),
            Item {
                id: 1,
                item_definition_id: "spiked_bat".to_string(),
            },
        );
        let mut item_definitions = HashMap::new();
        item_definitions.insert(
            "spiked_bat".to_string(),
            ItemDefinition {
                id: "spiked_bat".to_string(),
                name: "Spiked Bat".to_string(),
                description: Description::default(),
                use_type: ItemUseType::Passive,
                item_type: "weapon".to_string(),
                equipped_bonuses: EquippedBonuses {
                    attributes: vec![],
                    equipped: vec!["painful_smash".to_string()],
                },
                use_effects: vec![],
                alternate_names: vec![],
            },
        );
        let mut abilities = HashMap::new();
        abilities.insert(
            "painful_smash".to_string(),
            Ability {
                id: "painful_smash".to_string(),
                name: "Painful Smash".to_string(),
                description: Description::default(),
                effects: vec![],
                costs: vec![],
                modifiers: vec![],
                engagement_types: vec![EngagementType::Battle],
                role: AbilityRole::Attack,
                targets: vec![],
                action_text: None,
            },
        );

        // Not in innate_abilities — only reachable via combined_abilities (equipment grant).
        let resolved =
            queueable_ability(&character, &item_definitions, &abilities, "painful_smash");

        assert_eq!(resolved.map(|a| a.id), Some("painful_smash".to_string()));
    }

    #[test]
    fn queueable_ability_returns_none_for_unknown_id() {
        let character = Character::new(1, CharacterType::Player, test_location());
        let resolved = queueable_ability(
            &character,
            &HashMap::new(),
            &HashMap::new(),
            "does_not_exist",
        );
        assert!(resolved.is_none());
    }

    #[tokio::test]
    async fn stale_epoch_disconnect_preserves_reactivated_player() {
        let game_state = battle_state("machine:200").await;
        let db = Database::connect_in_memory().await.unwrap();
        // Simulate a reactivation that already bumped the epoch past the value the disconnect
        // was captured under, e.g. because the SSE cleanup task's mailbox push landed after
        // apply_activation's discard step already ran.
        game_state.bump_activation_epoch(1).await;
        game_state
            .mailboxes
            .push(
                1,
                Interaction::PlayerDisconnected {
                    client_id: "machine:200".to_string(),
                    epoch: 0,
                },
            )
            .await;

        process(&game_state, &db, 0).await;

        assert!(game_state.active_characters.read().await.contains_key(&1));
        assert!(
            game_state
                .active_players
                .read()
                .await
                .contains_key("machine:200")
        );
        assert_eq!(
            game_state.engagements.battles.find_for_entity(1).await,
            Some(1)
        );
    }

    #[tokio::test]
    async fn current_epoch_disconnect_tears_down_player() {
        let game_state = battle_state("machine:100").await;
        let db = Database::connect_in_memory().await.unwrap();
        game_state
            .mailboxes
            .push(
                1,
                Interaction::PlayerDisconnected {
                    client_id: "machine:100".to_string(),
                    epoch: 0,
                },
            )
            .await;

        process(&game_state, &db, 0).await;

        assert!(!game_state.active_characters.read().await.contains_key(&1));
        assert!(game_state.active_players.read().await.is_empty());
        assert_eq!(
            game_state.engagements.battles.find_for_entity(1).await,
            None
        );
    }
}
