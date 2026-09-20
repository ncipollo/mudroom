mod matching;

use std::sync::Arc;

use tracing;

use crate::game::component::Location;
use crate::game::player::Player;
use crate::game::{GameState, messaging};
use crate::persistence::Database;
use crate::persistence::room_feature_repo;
use matching::{InteractMatch, matching_features};

pub async fn process(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    verb: &str,
    target: &str,
) {
    let Some(location) = player_location(game_state, player).await else {
        return;
    };

    let matches = match matching_features(db, &location, verb, target).await {
        Ok(matches) => matches,
        Err(e) => {
            tracing::error!("Failed to load interact targets: {e}");
            return;
        }
    };

    respond_to_interact(game_state, db, player, verb, target, matches.as_slice()).await;
}

async fn player_location(game_state: &Arc<GameState>, player: &Player) -> Option<Location> {
    let characters = game_state.active_characters.read().await;
    characters
        .get(&player.entity_id)
        .map(|c| c.location.clone())
}

async fn respond_to_interact(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    verb: &str,
    target: &str,
    matches: &[InteractMatch],
) {
    match matches {
        [] => messaging::message(
            &game_state.message_tx,
            player.id,
            format!("You don't see a '{target}' here."),
        ),
        [feature_match] => apply_interact(game_state, db, player, verb, feature_match).await,
        _ => messaging::message(
            &game_state.message_tx,
            player.id,
            format!("Which '{target}' do you mean?"),
        ),
    }
}

async fn apply_interact(
    game_state: &Arc<GameState>,
    db: &Database,
    player: &Player,
    verb: &str,
    feature_match: &InteractMatch,
) {
    let Some(next_state) = &feature_match.next_state else {
        messaging::message(&game_state.message_tx, player.id, "Nothing happens.");
        return;
    };

    let update_result =
        room_feature_repo::update_state(db.pool(), feature_match.room_feature_id, next_state).await;
    if let Err(e) = update_result {
        tracing::error!("Failed to update feature state for interact: {e}");
        return;
    }

    messaging::message(
        &game_state.message_tx,
        player.id,
        interact_message(verb, &feature_match.name),
    );
}

fn interact_message(verb: &str, name: &str) -> String {
    if verb.eq_ignore_ascii_case("interact") {
        format!("You interact with the {name}.")
    } else {
        format!("You {} the {name}.", verb.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use tokio::sync::broadcast;

    use super::*;
    use crate::game::component::description::Description;
    use crate::game::entity::character::{Character, CharacterType};
    use crate::game::messaging::{Message, PlayerMessage};
    use crate::game::{Dungeon, FeatureState, Room, RoomFeature, World};
    use crate::persistence::{dungeon_repo, room_repo, world_repo};

    fn test_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    async fn setup_world(db: &Database) {
        world_repo::insert(db.pool(), &World::new("w1".to_string()))
            .await
            .unwrap();
        dungeon_repo::insert(db.pool(), &Dungeon::new("d1".to_string()), "w1")
            .await
            .unwrap();
        room_repo::insert(
            db.pool(),
            &Room::new("r1".to_string(), Description::new(None)),
            "d1",
        )
        .await
        .unwrap();
    }

    fn test_player(entity_id: i64) -> Player {
        Player {
            id: 1,
            client_id: "client".to_string(),
            name: "Hero".to_string(),
            entity_id,
        }
    }

    async fn game_state_with_character(db: &Database) -> (Arc<GameState>, Player) {
        setup_world(db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        let entity_id = 1;
        game_state.active_characters.write().await.insert(
            entity_id,
            Character::new(entity_id, CharacterType::Player, test_location()),
        );
        (game_state, test_player(entity_id))
    }

    fn chest_feature(alt_verbs: Vec<&str>, has_next_state: bool) -> RoomFeature {
        let mut states = HashMap::new();
        states.insert(
            "closed".to_string(),
            FeatureState {
                description: Description::new(Some("A closed chest.".to_string())),
                items: vec![],
                interact_script: None,
                interact_next_state: has_next_state.then(|| "open".to_string()),
            },
        );
        states.insert(
            "open".to_string(),
            FeatureState {
                description: Description::new(Some("An open chest.".to_string())),
                items: vec![],
                interact_script: None,
                interact_next_state: None,
            },
        );
        RoomFeature {
            id: "chest".to_string(),
            name: "Oak Chest".to_string(),
            default_state: "closed".to_string(),
            states,
            alt_verbs: alt_verbs.into_iter().map(str::to_string).collect(),
        }
    }

    async fn seed_feature(db: &Database, feature: &RoomFeature) -> i64 {
        room_feature_repo::upsert_definition(db.pool(), feature)
            .await
            .unwrap();
        let (id, _) = room_feature_repo::insert_placement_if_missing(
            db.pool(),
            &test_location(),
            &feature.id,
            "closed",
            &[],
        )
        .await
        .unwrap();
        id
    }

    /// Callers must subscribe before invoking `process` — a broadcast receiver only sees
    /// messages sent after it subscribes, so subscribing afterward would hang forever
    /// waiting on a message that already went out.
    async fn recv_message(rx: &mut broadcast::Receiver<PlayerMessage>) -> String {
        let msg = rx.recv().await.unwrap();
        match msg.message {
            Message::Complete { content, .. } => content,
            other => panic!("expected Complete message, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn interact_advances_state_and_persists_it() {
        let db = Database::connect_in_memory().await.unwrap();
        let (game_state, player) = game_state_with_character(&db).await;
        let room_feature_id = seed_feature(&db, &chest_feature(vec![], true)).await;

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, "interact", "oak chest").await;
        let content = recv_message(&mut rx).await;

        assert_eq!(content, "You interact with the Oak Chest.");

        let placed = room_feature_repo::find_by_location(db.pool(), &test_location())
            .await
            .unwrap();
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].id, room_feature_id);
        assert_eq!(placed[0].current_state, "open");
    }

    #[tokio::test]
    async fn declared_alt_verb_advances_state_with_a_flavorful_message() {
        let db = Database::connect_in_memory().await.unwrap();
        let (game_state, player) = game_state_with_character(&db).await;
        seed_feature(&db, &chest_feature(vec!["open"], true)).await;

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, "open", "oak chest").await;
        let content = recv_message(&mut rx).await;

        assert_eq!(content, "You open the Oak Chest.");
    }

    #[tokio::test]
    async fn undeclared_alt_verb_is_treated_as_no_match() {
        let db = Database::connect_in_memory().await.unwrap();
        let (game_state, player) = game_state_with_character(&db).await;
        seed_feature(&db, &chest_feature(vec![], true)).await;

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, "push", "oak chest").await;
        let content = recv_message(&mut rx).await;

        assert_eq!(content, "You don't see a 'oak chest' here.");
    }

    #[tokio::test]
    async fn no_match_when_no_feature_shares_the_name() {
        let db = Database::connect_in_memory().await.unwrap();
        let (game_state, player) = game_state_with_character(&db).await;

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, "interact", "oak chest").await;
        let content = recv_message(&mut rx).await;

        assert_eq!(content, "You don't see a 'oak chest' here.");
    }

    #[tokio::test]
    async fn ambiguous_match_when_two_features_share_a_name() {
        let db = Database::connect_in_memory().await.unwrap();
        let (game_state, player) = game_state_with_character(&db).await;
        let mut second = chest_feature(vec![], true);
        second.id = "chest_2".to_string();
        seed_feature(&db, &chest_feature(vec![], true)).await;
        seed_feature(&db, &second).await;

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, "interact", "oak chest").await;
        let content = recv_message(&mut rx).await;

        assert_eq!(content, "Which 'oak chest' do you mean?");
    }

    #[tokio::test]
    async fn no_next_state_is_a_no_op() {
        let db = Database::connect_in_memory().await.unwrap();
        let (game_state, player) = game_state_with_character(&db).await;
        seed_feature(&db, &chest_feature(vec![], false)).await;

        let mut rx = game_state.message_tx.subscribe();
        process(&game_state, &db, &player, "interact", "oak chest").await;
        let content = recv_message(&mut rx).await;

        assert_eq!(content, "Nothing happens.");
    }
}
