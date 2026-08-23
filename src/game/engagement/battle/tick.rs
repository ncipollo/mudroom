use std::collections::HashMap;
use std::sync::Arc;

use crate::game::GameState;

use super::message::broadcast::{self, BattleUpdateParams};
use super::message::tick_message::assemble_tick_messages;
use super::resolution::{ability, innate};
use super::{BattleMessage, BattlePhase, BattleTick, QueuedAbility};
use super::{attribute_snapshot, death, timer, victory};

/// Advances all active battle engagements one tick and handles the full lifecycle:
/// phase state machine → effect resolution → dead-character removal → engagement conclusion.
/// This is the single entry point for battle processing; no battle-specific logic escapes
/// into the engagement orchestration layer.
pub async fn process_ticks(game_state: &Arc<GameState>, max_engage_ticks: u64) {
    let battle_results = game_state
        .engagements
        .battles
        .tick_all(max_engage_ticks)
        .await;
    for result in battle_results {
        handle_tick(game_state, result, max_engage_ticks).await;
    }
}

/// Dispatches the work due for a completed battle tick based on `result.completed_phase` —
/// effect application, ability resolution, or death detection — then broadcasts the full battle
/// state to all player participants, which happens every tick regardless of phase. If the tick
/// just brought the battle to `Concluded`, also handles battle-ended cleanup.
async fn handle_tick(game_state: &Arc<GameState>, result: BattleTick, max_engage_ticks: u64) {
    let engagement_id = result.engagement_id;
    let all_ids = result.all_participant_ids.clone();
    let entity_names = collect_entity_names(game_state, &all_ids).await;

    // Run whatever work is due for the phase that just completed (if any).
    let (cast_messages, dead_ids) = dispatch_phase_work(
        game_state,
        engagement_id,
        &result.completed_phase,
        result.turn_count,
        result.resolution_queue,
        &all_ids,
        &entity_names,
    )
    .await;

    // Combine this tick's messages into one ordered log for the state broadcast below.
    let all_tick_messages = assemble_tick_messages(
        &result.messages,
        &result.pending_actions,
        &cast_messages,
        &dead_ids,
        &entity_names,
    );

    // Broadcast the current battle state to every player, every tick, regardless of phase.
    let player_pairs = find_participant_player_ids(game_state, &all_ids).await;
    let countdown_ticks = max_engage_ticks.saturating_sub(result.ticks_in_phase);
    let tick_rate_ms = game_state.mud_config.game_loop.tick_rate_ms;
    let params = BattleUpdateParams {
        engagement_id,
        factions: result.factions,
        participants: result.participants,
        phase: result.phase.clone(),
        messages: all_tick_messages,
        countdown_secs: timer::ticks_to_secs(countdown_ticks, tick_rate_ms),
        max_turn_secs: timer::ticks_to_secs(max_engage_ticks, tick_rate_ms),
    };
    let update = broadcast::build_battle_update(game_state, params).await;
    broadcast::broadcast_update(game_state, &player_pairs, &update).await;

    // If this tick just concluded the battle, run end-of-battle cleanup and tear it down.
    if result.phase == BattlePhase::Concluded {
        victory::handle_battle_ended(game_state, engagement_id, &all_ids, &player_pairs).await;
        game_state.engagements.battles.conclude(engagement_id).await;
        game_state.engagements.battles.remove(engagement_id).await;
    }
}

/// Runs the work gated on `completed_phase`: attribute resets (`ResetAttributes`), over-time
/// effect application (`ApplyEffects`), ability effect resolution (`ResolveAbilities`), or
/// dead-character detection and removal (`ResolveEntityState`). Returns the messages and dead character
/// ids produced, if any.
async fn dispatch_phase_work(
    game_state: &Arc<GameState>,
    engagement_id: i64,
    completed_phase: &BattlePhase,
    turn_count: u64,
    resolution_queue: Vec<QueuedAbility>,
    all_ids: &[i64],
    entity_names: &HashMap<i64, String>,
) -> (Vec<BattleMessage>, Vec<i64>) {
    let mut cast_messages = Vec::new();
    let mut dead_ids = Vec::new();
    match completed_phase {
        BattlePhase::ResetAttributes { .. } => {
            attribute_snapshot::reset_turn_start_attributes(
                game_state,
                engagement_id,
                all_ids,
                &game_state.attribute_config,
            )
            .await;
        }
        BattlePhase::ApplyEffects { .. } => {
            innate::apply_innate_effects(game_state, all_ids, turn_count, &mut cast_messages).await;
        }
        BattlePhase::ResolveAbilities => {
            ability::apply_battle_effects(
                game_state,
                resolution_queue,
                entity_names,
                &mut cast_messages,
            )
            .await;
        }
        BattlePhase::ResolveEntityState => {
            dead_ids =
                death::detect_dead_entities(game_state, all_ids, &game_state.attribute_config)
                    .await;
            game_state
                .engagements
                .battles
                .update_participants(engagement_id, &dead_ids)
                .await;
        }
        _ => {}
    }
    (cast_messages, dead_ids)
}

async fn collect_entity_names(
    game_state: &Arc<GameState>,
    entity_ids: &[i64],
) -> HashMap<i64, String> {
    let entities = game_state.active_characters.read().await;
    entity_ids
        .iter()
        .map(|&eid| {
            let name = entities
                .get(&eid)
                .map(|e| e.name.clone())
                .unwrap_or_else(|| format!("Character {eid}"));
            (eid, name)
        })
        .collect()
}

async fn find_participant_player_ids(
    game_state: &Arc<GameState>,
    entity_ids: &[i64],
) -> Vec<(i64, i64)> {
    let players = game_state.active_players.read().await;
    players
        .values()
        .filter(|p| entity_ids.contains(&p.entity_id))
        .map(|p| (p.id, p.entity_id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::Attribute;
    use crate::game::component::Location;
    use crate::game::component::effect::{
        Effect, EffectDescription, EffectScope, EffectType, TriggerInfo,
    };
    use crate::game::entity::character::{Character, CharacterType};

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    /// `AttributeShield` effects are re-resolved (and re-pushed onto `active_effects`) every time
    /// they're passed through `resolve_effects`, which makes the count of `active_effects` a
    /// reliable observable proxy for "how many times has this over-time effect actually fired."
    fn warded_entity(id: i64, name: &str) -> Character {
        let mut character = Character::new(id, CharacterType::Player, test_location());
        character.name = name.to_string();
        character.attributes.insert(
            "hp".to_string(),
            Attribute::new("hp".to_string(), 0, 100, 100),
        );
        character.active_effects.push(Effect {
            name: "warding".to_string(),
            effect_type: EffectType::AttributeShield {
                attribute_id: "hp".to_string(),
                absorb_amount: 5,
            },
            trigger_info: TriggerInfo::OverTime {
                start: 1,
                end: None,
                rate: 1,
            },
            description: EffectDescription::default(),
            scope: EffectScope::Battle,
        });
        character
    }

    fn plain_entity(id: i64, name: &str) -> Character {
        let mut character = Character::new(id, CharacterType::Enemy, test_location());
        character.name = name.to_string();
        character.attributes.insert(
            "hp".to_string(),
            Attribute::new("hp".to_string(), 0, 100, 100),
        );
        character
    }

    fn entity_with_strength(id: i64, name: &str) -> Character {
        let mut character = Character::new(id, CharacterType::Player, test_location());
        character.name = name.to_string();
        character.attributes.insert(
            "hp".to_string(),
            Attribute::new("hp".to_string(), 0, 100, 100),
        );
        character.attributes.insert(
            "strength".to_string(),
            Attribute::new("strength".to_string(), 1, 20, 10),
        );
        character
    }

    #[tokio::test]
    async fn apply_effects_fires_once_per_turn_while_dwelling_in_declare_phases() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        {
            let mut entities = game_state.active_characters.write().await;
            entities.insert(1, warded_entity(1, "Hero"));
            entities.insert(2, plain_entity(2, "Goblin"));
        }
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);
        participants.insert("enemy".to_string(), vec![2]);
        let factions = vec!["player".to_string(), "enemy".to_string()];
        game_state
            .engagements
            .add_battle("room".to_string(), factions, participants)
            .await;

        let max_engage_ticks = 50;
        // ResetAttributes -> AnnounceState -> ApplyEffects -> DeclareAttacks: the ApplyEffects
        // phase completes on the 3rd tick, applying the over-time effect exactly once.
        for _ in 0..3 {
            process_ticks(&game_state, max_engage_ticks).await;
        }
        // Dwell in DeclareAttacks across many raw ticks without submitting actions; the effect
        // must not re-fire since completed_phase stays DeclareAttacks, not ApplyEffects.
        for _ in 0..10 {
            process_ticks(&game_state, max_engage_ticks).await;
        }

        let entities = game_state.active_characters.read().await;
        // The over-time effect is resolved and re-pushed onto active_effects each time it
        // fires; if it fired once, the count grows from 1 to 2 — not once per dwelling tick.
        assert_eq!(entities[&1].active_effects.len(), 2);
    }

    #[tokio::test]
    async fn reset_attributes_phase_restores_battle_start_and_leaves_hp_alone() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        {
            let mut entities = game_state.active_characters.write().await;
            entities.insert(1, entity_with_strength(1, "Hero"));
            entities.insert(2, plain_entity(2, "Goblin"));
        }
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);
        participants.insert("enemy".to_string(), vec![2]);
        let factions = vec!["player".to_string(), "enemy".to_string()];
        let engagement_id = game_state
            .engagements
            .add_battle("room".to_string(), factions, participants)
            .await;
        attribute_snapshot::capture_battle_start(&game_state, engagement_id, &[1, 2]).await;

        let max_engage_ticks = 1;
        // Completes the initial ResetAttributes{player} phase — a no-op reset since nothing has
        // mutated live attributes yet.
        process_ticks(&game_state, max_engage_ticks).await;

        // Simulate a mid-turn effect write, exactly as `resolution/effect.rs` would do.
        {
            let mut entities = game_state.active_characters.write().await;
            let character = entities.get_mut(&1).unwrap();
            character
                .attributes
                .get_mut("strength")
                .unwrap()
                .current_value = 3;
            character.attributes.get_mut("hp").unwrap().current_value = 40;
        }

        // AnnounceState -> ApplyEffects -> DeclareAttacks -> DeclareDefense -> ResolveAbilities
        // -> ResolveEntityState -> Cleanup -> VictoryCheck -> ResetAttributes{enemy} (9 ticks),
        // then one more tick dispatches the ResetAttributes{enemy} phase's reset work.
        for _ in 0..10 {
            process_ticks(&game_state, max_engage_ticks).await;
        }

        let entities = game_state.active_characters.read().await;
        assert_eq!(entities[&1].attributes["strength"].current_value, 10);
        assert_eq!(entities[&1].attributes["hp"].current_value, 40);
    }
}
