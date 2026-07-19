pub mod conversation;
pub mod help;
pub mod lifecycle;
pub mod look;
pub mod movement;
pub mod room_threats;

use std::sync::Arc;

use tracing;

use crate::game::component::interaction::Movement;
use crate::game::engagement::TurnAction;
use crate::game::engagement::battle;
use crate::game::messaging;
use crate::game::player::Player;
use crate::game::{GameState, Interaction};
use crate::persistence::Database;

pub async fn process(game_state: &Arc<GameState>, db: &Database, tick: u64) {
    tracing::debug!("Processing interactions tick={tick}");

    let players: Vec<Player> = game_state
        .active_players
        .read()
        .await
        .values()
        .cloned()
        .collect();

    for player in players {
        process_player(game_state, db, &player).await;
    }
}

async fn process_player(game_state: &Arc<GameState>, db: &Database, player: &Player) {
    let interactions = game_state.mailboxes.drain(player.entity_id).await;
    for interaction in interactions {
        dispatch_interaction(game_state, db, player, interaction).await;
    }
}

async fn dispatch_interaction(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    interaction: Interaction,
) {
    match interaction {
        Interaction::Look => look::process(game_state, db, player).await,
        Interaction::Help => help::process(game_state, player).await,
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
        Interaction::PlayerDisconnected => {
            lifecycle::player_disconnected(game_state, player).await;
        }
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
        let entities = game_state.active_entities.read().await;
        let Some(entity) = entities.get(&player.entity_id) else {
            return;
        };
        let ability = entity
            .innate_abilities
            .iter()
            .find(|a| a.id == ability_id)
            .cloned();
        (ability, entity.attributes.clone())
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
        let entities = game_state.active_entities.read().await;
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
        let entities = game_state.active_entities.read().await;
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
        game_state.engagements.battles.conclude(engagement_id).await;
        game_state.engagements.battles.remove(engagement_id).await;
    }

    messaging::battle_ended(&game_state.message_tx, player.id, engagement_id);
}
