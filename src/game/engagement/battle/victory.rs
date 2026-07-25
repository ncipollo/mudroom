use std::sync::Arc;

use crate::game::component::effect::EffectScope;
use crate::game::{GameState, messaging};

use super::loot;

/// Handles a battle reaching `BattlePhase::Concluded`: resolves loot, clears battle-scoped
/// active effects from all participants, and notifies player participants the battle has ended.
pub(super) async fn handle_battle_ended(
    game_state: &Arc<GameState>,
    engagement_id: i64,
    all_participant_ids: &[i64],
    player_ids: &[i64],
) {
    loot::resolve_loot(all_participant_ids);
    clear_active_effects(game_state, all_participant_ids).await;
    for &pid in player_ids {
        messaging::battle_ended(&game_state.message_tx, pid, engagement_id);
    }
}

async fn clear_active_effects(game_state: &Arc<GameState>, entity_ids: &[i64]) {
    let mut entities = game_state.active_entities.write().await;
    for &id in entity_ids {
        if let Some(entity) = entities.get_mut(&id) {
            entity
                .active_effects
                .retain(|e| e.scope != EffectScope::Battle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::Location;
    use crate::game::component::effect::{Effect, EffectDescription, EffectType, TriggerInfo};
    use crate::game::entity::{Entity, EntityType};
    use tokio::sync::broadcast;

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    fn battle_scoped_effect() -> Effect {
        Effect {
            name: "poison".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -1,
            },
            trigger_info: TriggerInfo::OverTime {
                start: 0,
                end: None,
                rate: 1,
            },
            description: EffectDescription::default(),
            scope: EffectScope::Battle,
        }
    }

    fn world_scoped_effect() -> Effect {
        let mut effect = battle_scoped_effect();
        effect.name = "regeneration".to_string();
        effect.scope = EffectScope::World;
        effect
    }

    async fn game_state_with_entity(entity: Entity) -> Arc<GameState> {
        let game_state = Arc::new(GameState::load(None).unwrap());
        game_state
            .active_entities
            .write()
            .await
            .insert(entity.id, entity);
        game_state
    }

    #[tokio::test]
    async fn clear_active_effects_removes_only_battle_scoped_effects() {
        let mut entity = Entity::new(1, EntityType::Player, test_location());
        entity.active_effects = vec![battle_scoped_effect(), world_scoped_effect()];
        let game_state = game_state_with_entity(entity).await;

        clear_active_effects(&game_state, &[1]).await;

        let entities = game_state.active_entities.read().await;
        assert_eq!(entities[&1].active_effects.len(), 1);
        assert_eq!(entities[&1].active_effects[0].scope, EffectScope::World);
    }

    #[tokio::test]
    async fn clear_active_effects_ignores_missing_entities() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        clear_active_effects(&game_state, &[99]).await;
    }

    #[tokio::test]
    async fn handle_battle_ended_clears_effects_and_notifies_players() {
        let mut entity = Entity::new(1, EntityType::Player, test_location());
        entity.active_effects = vec![battle_scoped_effect()];
        let game_state = game_state_with_entity(entity).await;
        let mut receiver = game_state.message_tx.subscribe();

        handle_battle_ended(&game_state, 42, &[1], &[7]).await;

        let entities = game_state.active_entities.read().await;
        assert!(entities[&1].active_effects.is_empty());
        drop(entities);

        let message = receiver.try_recv();
        assert!(
            message.is_ok(),
            "expected a battle_ended message to be sent"
        );
    }

    #[tokio::test]
    async fn handle_battle_ended_sends_nothing_when_no_players() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let mut receiver = game_state.message_tx.subscribe();

        handle_battle_ended(&game_state, 42, &[], &[]).await;

        assert!(matches!(
            receiver.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
    }
}
