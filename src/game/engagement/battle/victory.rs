use std::sync::Arc;

use crate::game::component::effect::EffectScope;
use crate::game::{GameState, messaging};

use super::{attribute_snapshot, loot};

/// Handles a battle reaching `BattlePhase::Concluded`: resolves loot, resets `EndOfEngagement`
/// attributes, clears battle-scoped active effects from all participants, and notifies player
/// participants the battle has ended.
pub(super) async fn handle_battle_ended(
    game_state: &Arc<GameState>,
    engagement_id: i64,
    all_participant_ids: &[i64],
    players: &[(i64, i64)],
) {
    loot::resolve_loot(all_participant_ids);
    attribute_snapshot::reset_end_of_engagement_attributes(
        game_state,
        engagement_id,
        all_participant_ids,
        &game_state.attribute_config,
    )
    .await;
    clear_active_effects(game_state, all_participant_ids).await;
    for &(player_id, entity_id) in players {
        messaging::battle_ended(&game_state.message_tx, player_id, engagement_id);
        sync_player_stats(game_state, player_id, entity_id).await;
    }
}

/// Pushes `entity_id`'s current HP/MP to `player_id`, so the world-view status bar reflects the
/// character's post-battle condition as soon as they leave the battle screen.
async fn sync_player_stats(game_state: &Arc<GameState>, player_id: i64, entity_id: i64) {
    let entities = game_state.active_characters.read().await;
    let Some(character) = entities.get(&entity_id) else {
        return;
    };
    let hp_attr_id = messaging::hp_attribute_id(&game_state.attribute_config);
    let mp_attr_id = messaging::mp_attribute_id(&game_state.attribute_config);
    let (hp_current, hp_max) = character.attribute_range(&hp_attr_id);
    let (mp_current, mp_max) = character.attribute_range(&mp_attr_id);
    messaging::player_stats_updated(
        &game_state.message_tx,
        player_id,
        hp_current,
        hp_max,
        mp_current,
        mp_max,
    );
}

async fn clear_active_effects(game_state: &Arc<GameState>, entity_ids: &[i64]) {
    let mut entities = game_state.active_characters.write().await;
    for &id in entity_ids {
        if let Some(character) = entities.get_mut(&id) {
            character
                .active_effects
                .retain(|e| e.scope != EffectScope::Battle);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::game::component::attribute_definition::ResetCondition;
    use crate::game::component::effect::{Effect, EffectDescription, EffectType, TriggerInfo};
    use crate::game::component::{Attribute, Location};
    use crate::game::entity::character::{Character, CharacterType};
    use crate::game::messaging::Message;
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
    async fn clear_active_effects_removes_only_battle_scoped_effects() {
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character.active_effects = vec![battle_scoped_effect(), world_scoped_effect()];
        let game_state = game_state_with_entity(character).await;

        clear_active_effects(&game_state, &[1]).await;

        let entities = game_state.active_characters.read().await;
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
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character.active_effects = vec![battle_scoped_effect()];
        let game_state = game_state_with_entity(character).await;
        let mut receiver = game_state.message_tx.subscribe();

        handle_battle_ended(&game_state, 42, &[1], &[(7, 1)]).await;

        let entities = game_state.active_characters.read().await;
        assert!(entities[&1].active_effects.is_empty());
        drop(entities);

        let message = receiver.try_recv();
        assert!(
            message.is_ok(),
            "expected a battle_ended message to be sent"
        );
    }

    #[tokio::test]
    async fn handle_battle_ended_syncs_player_stats() {
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character.attributes.insert(
            "hp".to_string(),
            Attribute::new("hp".to_string(), 0, 100, 40),
        );
        character.attributes.insert(
            "mp".to_string(),
            Attribute::new("mp".to_string(), 0, 50, 20),
        );
        let game_state = game_state_with_entity(character).await;
        let mut receiver = game_state.message_tx.subscribe();

        handle_battle_ended(&game_state, 42, &[1], &[(7, 1)]).await;

        // First message is battle_ended; the stats sync follows it.
        receiver
            .try_recv()
            .expect("expected a battle_ended message");
        let msg = receiver
            .try_recv()
            .expect("expected a player_stats_updated message");
        assert_eq!(msg.player_id, 7);
        assert!(matches!(
            msg.message,
            Message::PlayerStatsUpdated {
                hp_current: 40,
                hp_max: 100,
                mp_current: 20,
                mp_max: 50,
            }
        ));
    }

    #[tokio::test]
    async fn handle_battle_ended_resets_end_of_engagement_and_leaves_hp() {
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character.attributes.insert(
            "hp".to_string(),
            Attribute::new("hp".to_string(), 0, 100, 100),
        );
        character.attributes.insert(
            "strength".to_string(),
            Attribute::new("strength".to_string(), 1, 20, 10),
        );

        // `AttributeConfig::default_config` has no `EndOfEngagement` attributes (only `Never`
        // and `EachEngagementTurn`), so re-tag "strength" to exercise this reset.
        let mut game_state = GameState::load(None).unwrap();
        if let Some(def) = game_state
            .attribute_config
            .attributes
            .iter_mut()
            .find(|d| d.id == "strength")
        {
            def.reset_condition = ResetCondition::EndOfEngagement;
        }
        let game_state = Arc::new(game_state);
        game_state
            .active_characters
            .write()
            .await
            .insert(character.id, character);
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);
        let engagement_id = game_state
            .engagements
            .add_battle("room".to_string(), vec!["player".to_string()], participants)
            .await;

        attribute_snapshot::capture_battle_start(&game_state, engagement_id, &[1]).await;

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

        handle_battle_ended(&game_state, engagement_id, &[1], &[]).await;

        let entities = game_state.active_characters.read().await;
        assert_eq!(entities[&1].attributes["strength"].current_value, 10);
        assert_eq!(entities[&1].attributes["hp"].current_value, 40);
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
