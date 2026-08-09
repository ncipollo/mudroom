use std::sync::Arc;

use crate::game::GameState;
use crate::game::component::effect::{Effect, TriggerInfo};
use crate::game::engagement::battle::BattleMessage;

use super::effect::resolve_effects;

/// Applies triggered over-time ("innate") effects for the given participants and sweeps any
/// `OverTime` effects whose `end` has passed. Only call this when the current battle tick has
/// just completed the `ApplyEffects` phase — `turn_count` stays fixed for a whole faction turn,
/// so calling this on every raw tick would re-fire matching effects once per raw tick instead of
/// once per turn.
pub(in crate::game::engagement::battle) async fn apply_innate_effects(
    game_state: &Arc<GameState>,
    participant_ids: &[i64],
    turn_count: u64,
    messages: &mut Vec<BattleMessage>,
) {
    let mut entities = game_state.active_characters.write().await;
    for &entity_id in participant_ids {
        let Some(character) = entities.get(&entity_id) else {
            continue;
        };
        let entity_name = character.name.clone();
        let triggered: Vec<Effect> = character
            .active_effects
            .iter()
            .filter(|e| is_over_time_triggered(&e.trigger_info, turn_count))
            .cloned()
            .collect();
        let expired_names: Vec<String> = character
            .active_effects
            .iter()
            .filter(|e| is_over_time_expired(&e.trigger_info, turn_count))
            .map(|e| e.name.clone())
            .collect();

        if !triggered.is_empty() {
            let applied = resolve_effects(entity_id, triggered, &mut entities);
            messages.extend(applied);
        }

        if !expired_names.is_empty()
            && let Some(character) = entities.get_mut(&entity_id)
        {
            character
                .active_effects
                .retain(|e| !is_over_time_expired(&e.trigger_info, turn_count));
        }
        for effect_name in expired_names {
            messages.push(BattleMessage::EffectExpired {
                entity_name: entity_name.clone(),
                effect_name,
            });
        }
    }
}

fn is_over_time_triggered(trigger: &TriggerInfo, turn_count: u64) -> bool {
    match trigger {
        TriggerInfo::OverTime { start, end, rate } => {
            turn_count >= *start
                && end.is_none_or(|e| turn_count < e)
                && (turn_count - start).is_multiple_of(*rate)
        }
        TriggerInfo::Once => false,
    }
}

fn is_over_time_expired(trigger: &TriggerInfo, turn_count: u64) -> bool {
    matches!(trigger, TriggerInfo::OverTime { end: Some(end), .. } if turn_count >= *end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::Attribute;
    use crate::game::component::Location;
    use crate::game::component::effect::{EffectDescription, EffectScope, EffectType};
    use crate::game::entity::{Character, CharacterType};

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    fn hp_attribute(current: i64) -> Attribute {
        Attribute::new("hp".to_string(), 0, 100, current)
    }

    fn over_time_effect(value: i64, start: u64, end: Option<u64>, rate: u64) -> Effect {
        Effect {
            name: "poison".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value,
            },
            trigger_info: TriggerInfo::OverTime { start, end, rate },
            description: EffectDescription::default(),
            scope: EffectScope::Battle,
        }
    }

    /// `AttributeShield` effects are re-resolved (and re-pushed onto `active_effects`) every time
    /// they're passed through `resolve_effects`, unlike `AttributeUpdate` — which only applies
    /// for `Once`-triggered effects (see `resolve_effects` docs). That makes shield effects a
    /// useful observable proxy for "was this over-time effect actually resolved this call."
    fn over_time_shield_effect(
        absorb_amount: i64,
        start: u64,
        end: Option<u64>,
        rate: u64,
    ) -> Effect {
        Effect {
            name: "warding".to_string(),
            effect_type: EffectType::AttributeShield {
                attribute_id: "hp".to_string(),
                absorb_amount,
            },
            trigger_info: TriggerInfo::OverTime { start, end, rate },
            description: EffectDescription::default(),
            scope: EffectScope::Battle,
        }
    }

    async fn game_state_with_entity(character: Character) -> Arc<GameState> {
        let game_state = Arc::new(GameState::load(None).unwrap());
        game_state
            .active_characters
            .write()
            .await
            .insert(character.id, character);
        game_state
    }

    #[tokio::test]
    async fn apply_innate_effects_resolves_triggered_over_time_effect() {
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character
            .attributes
            .insert("hp".to_string(), hp_attribute(100));
        character
            .active_effects
            .push(over_time_shield_effect(5, 0, None, 1));
        let game_state = game_state_with_entity(character).await;

        let mut messages = Vec::new();
        apply_innate_effects(&game_state, &[1], 0, &mut messages).await;

        let entities = game_state.active_characters.read().await;
        // The triggered effect is resolved and re-pushed onto active_effects, proving it
        // actually ran (rather than being silently skipped by the trigger-gate check).
        assert_eq!(entities[&1].active_effects.len(), 2);
    }

    #[tokio::test]
    async fn apply_innate_effects_removes_expired_over_time_effect_and_emits_message() {
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character.name = "Goblin".to_string();
        character
            .attributes
            .insert("hp".to_string(), hp_attribute(100));
        character
            .active_effects
            .push(over_time_effect(-1, 0, Some(3), 1));
        let game_state = game_state_with_entity(character).await;

        let mut messages = Vec::new();
        apply_innate_effects(&game_state, &[1], 3, &mut messages).await;

        let entities = game_state.active_characters.read().await;
        assert!(entities[&1].active_effects.is_empty());
        assert!(messages.contains(&BattleMessage::EffectExpired {
            entity_name: "Goblin".to_string(),
            effect_name: "poison".to_string(),
        }));
    }

    #[tokio::test]
    async fn apply_innate_effects_still_triggers_on_the_tick_before_expiry() {
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character
            .attributes
            .insert("hp".to_string(), hp_attribute(100));
        character
            .active_effects
            .push(over_time_shield_effect(5, 0, Some(3), 1));
        let game_state = game_state_with_entity(character).await;

        let mut messages = Vec::new();
        apply_innate_effects(&game_state, &[1], 2, &mut messages).await;

        let entities = game_state.active_characters.read().await;
        assert_eq!(entities[&1].active_effects.len(), 2);
        assert!(messages.is_empty());
    }
}
