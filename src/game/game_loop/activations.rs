use std::sync::Arc;

use crate::game::engagement::battle;
use crate::game::game_state::PendingActivation;
use crate::game::interaction::room_threats;
use crate::game::player::Player;
use crate::game::{GameState, Interaction, messaging};
use crate::persistence::Database;

pub async fn process(game_state: &Arc<GameState>, db: &Database) {
    let pending = game_state.drain_pending_activations().await;
    for activation in pending {
        apply_activation(game_state, db, activation).await;
    }
}

async fn apply_activation(
    game_state: &Arc<GameState>,
    db: &Database,
    activation: PendingActivation,
) {
    let room_id = activation.character.location.room_id.clone();
    let entity_id = activation.player.entity_id;
    let client_id = activation.client_id.clone();
    let player = activation.player.clone();

    register_in_game_state(game_state, db, activation).await;
    reset_disconnect_state(game_state, entity_id, &client_id).await;
    reconcile_battle_state(game_state, &player).await;

    // Queued directly into the mailbox (rather than left to the client to request over
    // `/interactions`) because activation and this tick's interaction processing happen
    // atomically here; a client-driven request could race the activation and 404 before
    // `active_players` is populated.
    game_state
        .mailboxes
        .push(entity_id, Interaction::Look)
        .await;
    game_state
        .mailboxes
        .push(entity_id, Interaction::CheckRoomThreats { room_id })
        .await;
}

async fn register_in_game_state(
    game_state: &Arc<GameState>,
    db: &Database,
    activation: PendingActivation,
) {
    game_state
        .active_characters
        .write()
        .await
        .insert(activation.character.id, activation.character);

    evict_stale_client_registrations(
        game_state,
        activation.player.entity_id,
        &activation.client_id,
    )
    .await;
    game_state
        .active_players
        .write()
        .await
        .insert(activation.client_id, activation.player);

    if let Err(e) = game_state.sync_active_characters(db.pool()).await {
        tracing::error!(error = %e, "Failed to sync active entities on player activation");
    }
}

/// Removes any existing `active_players` entry for `entity_id` registered under a different
/// client_id. A reconnect under a new client_id (e.g. a new process) can otherwise leave a stale
/// entry behind if the old client_id's disconnect was never processed — leaving two entries
/// pointing at the same character. That stale entry would be indistinguishable, by character-scoped
/// activation epoch alone, from the fresh one when a disconnect eventually gets dispatched for
/// it, risking exactly the character it aliases getting torn down out from under the live client.
async fn evict_stale_client_registrations(
    game_state: &Arc<GameState>,
    entity_id: i64,
    new_client_id: &str,
) {
    game_state
        .active_players
        .write()
        .await
        .retain(|client_id, player| player.entity_id != entity_id || client_id == new_client_id);
}

/// Bumps the character's activation epoch and discards any already-queued stale disconnects. The
/// epoch bump is the authoritative guard: any `PlayerDisconnected` queued by a still-in-flight
/// SSE cleanup task (captured under the prior epoch) is stale even if it lands in the mailbox
/// after the discard below already ran — see `dispatch_player_disconnected`.
async fn reset_disconnect_state(game_state: &Arc<GameState>, entity_id: i64, client_id: &str) {
    let epoch = game_state.bump_activation_epoch(entity_id).await;
    let discarded = discard_stale_disconnects(game_state, entity_id).await;
    tracing::info!(
        entity_id,
        client_id,
        epoch,
        discarded_stale_disconnects = discarded,
        "player activated"
    );
}

/// A disconnect queued before this (re)activation is stale: the player is back, so the queued
/// teardown must not run. Discard any pending `PlayerDisconnected` already sitting in the
/// mailbox while preserving everything else, returning how many were discarded. This is a
/// best-effort optimization — the authoritative guard against a disconnect queued *after* this
/// point is the epoch check in `dispatch_player_disconnected`.
async fn discard_stale_disconnects(game_state: &Arc<GameState>, entity_id: i64) -> usize {
    let mut discarded = 0;
    game_state
        .mailboxes
        .retain(entity_id, |i| {
            let is_stale = matches!(i, Interaction::PlayerDisconnected { .. });
            if is_stale {
                discarded += 1;
            }
            !is_stale
        })
        .await;
    discarded
}

/// If the character is still a participant in an active battle (e.g. the disconnect teardown was
/// skipped because the player rejoined), resend `BattleStarted` so the client re-enters battle
/// mode with the current state.
async fn reconcile_battle_state(game_state: &Arc<GameState>, player: &Player) {
    let Some(engagement_id) = game_state
        .engagements
        .battles
        .find_for_entity(player.entity_id)
        .await
    else {
        return;
    };
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::Location;
    use crate::game::entity::{Character, CharacterType};
    use crate::game::messaging::Message;
    use crate::game::player::Player;
    use std::collections::HashMap;

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    fn test_activation(client_id: &str) -> PendingActivation {
        PendingActivation {
            character: Character::new(1, CharacterType::Player, test_location()),
            player: Player {
                id: 1,
                client_id: client_id.to_string(),
                name: "Hero".to_string(),
                entity_id: 1,
            },
            client_id: client_id.to_string(),
        }
    }

    #[tokio::test]
    async fn activation_evicts_stale_registration_under_old_client_id() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let db = Database::connect_in_memory().await.unwrap();
        // Simulate a reconnect under a new client_id (e.g. a new process) while the old
        // client_id's registration for the same character was never cleaned up.
        game_state.active_players.write().await.insert(
            "old-client".to_string(),
            Player {
                id: 1,
                client_id: "old-client".to_string(),
                name: "Hero".to_string(),
                entity_id: 1,
            },
        );
        let activation = test_activation("new-client");
        game_state
            .push_pending_activation(
                activation.character,
                activation.player,
                "new-client".to_string(),
            )
            .await;

        process(&game_state, &db).await;

        let players = game_state.active_players.read().await;
        assert!(
            !players.contains_key("old-client"),
            "stale registration under the old client_id must be evicted"
        );
        assert!(players.contains_key("new-client"));
        assert_eq!(players.len(), 1);
    }

    #[tokio::test]
    async fn activation_discards_stale_player_disconnected() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let db = Database::connect_in_memory().await.unwrap();
        game_state
            .mailboxes
            .push(
                1,
                Interaction::PlayerDisconnected {
                    client_id: "m:1".to_string(),
                    epoch: 0,
                },
            )
            .await;
        game_state.mailboxes.push(1, Interaction::Look).await;
        game_state
            .push_pending_activation(
                test_activation("m:1").character,
                test_activation("m:1").player,
                "m:1".to_string(),
            )
            .await;

        process(&game_state, &db).await;

        let remaining = game_state.mailboxes.drain(1).await;
        assert!(
            remaining
                .iter()
                .all(|i| !matches!(i, Interaction::PlayerDisconnected { .. })),
            "stale PlayerDisconnected should be discarded, got {remaining:?}"
        );
        assert!(
            remaining.iter().any(|i| matches!(i, Interaction::Look)),
            "other interactions should be preserved, got {remaining:?}"
        );
        assert!(game_state.active_players.read().await.contains_key("m:1"));
        assert!(game_state.active_characters.read().await.contains_key(&1));
    }

    #[tokio::test]
    async fn activation_during_active_battle_resends_battle_started() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let db = Database::connect_in_memory().await.unwrap();
        game_state
            .active_characters
            .write()
            .await
            .insert(2, Character::new(2, CharacterType::Enemy, test_location()));
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
        let mut rx = game_state.message_tx.subscribe();
        let activation = test_activation("m:1");
        game_state
            .push_pending_activation(activation.character, activation.player, "m:1".to_string())
            .await;

        process(&game_state, &db).await;

        let msg = rx.try_recv().expect("expected a BattleStarted message");
        assert_eq!(msg.player_id, 1);
        assert!(
            matches!(msg.message, Message::BattleStarted(_)),
            "expected BattleStarted, got {:?}",
            msg.message
        );
    }

    #[tokio::test]
    async fn activation_outside_battle_sends_no_battle_started() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let db = Database::connect_in_memory().await.unwrap();
        let mut rx = game_state.message_tx.subscribe();
        let activation = test_activation("m:1");
        game_state
            .push_pending_activation(activation.character, activation.player, "m:1".to_string())
            .await;

        process(&game_state, &db).await;

        assert!(rx.try_recv().is_err(), "expected no messages");
    }

    #[tokio::test]
    async fn activation_queues_look_before_check_room_threats() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let db = Database::connect_in_memory().await.unwrap();
        let activation = test_activation("m:1");
        game_state
            .push_pending_activation(activation.character, activation.player, "m:1".to_string())
            .await;

        process(&game_state, &db).await;

        let queued = game_state.mailboxes.drain(1).await;
        assert!(
            matches!(queued.first(), Some(Interaction::Look)),
            "expected Look to be queued first on activation, got {queued:?}"
        );
        assert!(
            queued
                .iter()
                .any(|i| matches!(i, Interaction::CheckRoomThreats { .. })),
            "expected CheckRoomThreats to still be queued, got {queued:?}"
        );
    }
}
