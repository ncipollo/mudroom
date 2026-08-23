use std::sync::Arc;

use crate::game::GameState;

use super::victory;

/// Ends a battle outside the normal tick-driven conclusion flow — e.g. a player disconnecting
/// or leaving reduces the battle to a single surviving faction. Runs the same end-of-battle
/// cleanup as a tick-driven conclusion (loot, `EndOfEngagement` attribute resets, battle-scoped
/// effect clearing, `BattleEnded` notifications), then tears the engagement down.
///
/// `departed_entity_ids` are entities already removed from the engagement (e.g. the player who
/// just left); they still receive attribute/effect cleanup but no `BattleEnded` notification —
/// the caller notifies them directly.
pub async fn end_battle(
    game_state: &Arc<GameState>,
    engagement_id: i64,
    departed_entity_ids: &[i64],
) {
    let Some(entity_ids) = game_state
        .engagements
        .battles
        .entity_ids(engagement_id)
        .await
    else {
        return;
    };
    let players = participant_players(game_state, &entity_ids).await;

    let mut cleanup_ids = entity_ids.clone();
    for &id in departed_entity_ids {
        if !cleanup_ids.contains(&id) {
            cleanup_ids.push(id);
        }
    }

    victory::handle_battle_ended(game_state, engagement_id, &cleanup_ids, &players).await;
    game_state.engagements.battles.conclude(engagement_id).await;
    game_state.engagements.battles.remove(engagement_id).await;
}

/// Returns `(player_id, entity_id)` pairs for every player among `entity_ids`.
async fn participant_players(game_state: &Arc<GameState>, entity_ids: &[i64]) -> Vec<(i64, i64)> {
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
    use crate::game::component::Location;
    use crate::game::component::effect::{
        Effect, EffectDescription, EffectScope, EffectType, TriggerInfo,
    };
    use crate::game::entity::character::{Character, CharacterType};
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

    fn test_player(id: i64, entity_id: i64) -> Player {
        Player {
            id,
            client_id: format!("client:{id}"),
            name: format!("Player{id}"),
            entity_id,
        }
    }

    async fn battle_game_state() -> Arc<GameState> {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let mut departing = Character::new(1, CharacterType::Player, test_location());
        departing.active_effects = vec![battle_scoped_effect()];
        let mut surviving = Character::new(2, CharacterType::Player, test_location());
        surviving.active_effects = vec![battle_scoped_effect()];
        {
            let mut entities = game_state.active_characters.write().await;
            entities.insert(1, departing);
            entities.insert(2, surviving);
            entities.insert(3, Character::new(3, CharacterType::Enemy, test_location()));
        }
        {
            let mut players = game_state.active_players.write().await;
            players.insert("client:11".to_string(), test_player(11, 1));
            players.insert("client:12".to_string(), test_player(12, 2));
        }
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1, 2]);
        participants.insert("enemy".to_string(), vec![3]);
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

    #[tokio::test]
    async fn end_battle_tears_down_battle_and_notifies_remaining_players() {
        let game_state = battle_game_state().await;
        // Simulate the departing player already removed from the engagement.
        let (engagement_id, _) = crate::game::engagement::battle::participants::remove_entity(
            &game_state.engagements.battles,
            1,
        )
        .await
        .unwrap();
        let mut rx = game_state.message_tx.subscribe();

        end_battle(&game_state, engagement_id, &[1]).await;

        assert_eq!(
            game_state.engagements.battles.find_for_entity(2).await,
            None,
            "battle should be torn down"
        );

        // The remaining player is notified; the departed player is not.
        let msg = rx.try_recv().expect("expected a BattleEnded message");
        assert_eq!(msg.player_id, 12);
        assert!(matches!(msg.message, Message::BattleEnded { .. }));

        // Also synced: the remaining player's post-battle HP/MP.
        let msg = rx
            .try_recv()
            .expect("expected a PlayerStatsUpdated message");
        assert_eq!(msg.player_id, 12);
        assert!(matches!(msg.message, Message::PlayerStatsUpdated { .. }));

        assert!(rx.try_recv().is_err(), "expected exactly two messages");

        // Battle-scoped effects are cleared from both survivors and departed entities.
        let entities = game_state.active_characters.read().await;
        assert!(entities[&1].active_effects.is_empty());
        assert!(entities[&2].active_effects.is_empty());
    }

    #[tokio::test]
    async fn end_battle_with_missing_engagement_is_a_no_op() {
        let game_state = battle_game_state().await;
        end_battle(&game_state, 999, &[]).await;
        // The real battle is untouched.
        assert_eq!(
            game_state.engagements.battles.find_for_entity(2).await,
            Some(1)
        );
    }
}
