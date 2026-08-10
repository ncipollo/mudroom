use std::collections::HashMap;
use std::sync::Arc;

use crate::game::GameState;
use crate::game::entity::character::Character;

pub(super) async fn build_participants(
    game_state: &Arc<GameState>,
    player_entity_id: i64,
    hostile_ids: &[i64],
) -> (Vec<String>, HashMap<String, Vec<i64>>) {
    let entities = game_state.active_characters.read().await;

    let mut participants: HashMap<String, Vec<i64>> = HashMap::new();
    for &entity_id in std::iter::once(&player_entity_id).chain(hostile_ids.iter()) {
        if let Some(character) = entities.get(&entity_id) {
            for faction in &character.factions {
                participants
                    .entry(faction.clone())
                    .or_default()
                    .push(entity_id);
            }
        }
    }

    let factions = ordered_factions(&entities, player_entity_id, &participants);
    (factions, participants)
}

pub(super) fn ordered_factions(
    entities: &HashMap<i64, Character>,
    player_entity_id: i64,
    participants: &HashMap<String, Vec<i64>>,
) -> Vec<String> {
    let mut factions: Vec<String> = Vec::new();
    if let Some(player_entity) = entities.get(&player_entity_id) {
        for faction in &player_entity.factions {
            factions.push(faction.clone());
        }
    }
    for faction in participants.keys() {
        if !factions.contains(faction) {
            factions.push(faction.clone());
        }
    }
    factions
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::game::GameState;
    use crate::game::component::location::Location;
    use crate::game::entity::character::{Character, CharacterType};

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    fn make_entity(id: i64, entity_type: CharacterType, factions: &[&str]) -> Character {
        let mut character = Character::new(id, entity_type, test_location());
        character.factions = factions.iter().map(|s| s.to_string()).collect();
        character
    }

    // ordered_factions tests

    #[test]
    fn ordered_factions_player_faction_comes_first() {
        let mut entities = HashMap::new();
        entities.insert(1, make_entity(1, CharacterType::Player, &["player"]));
        entities.insert(2, make_entity(2, CharacterType::Enemy, &["enemy"]));

        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);
        participants.insert("enemy".to_string(), vec![2]);

        let factions = ordered_factions(&entities, 1, &participants);
        assert_eq!(factions[0], "player");
        assert!(factions.contains(&"enemy".to_string()));
    }

    #[test]
    fn ordered_factions_no_duplicates() {
        let mut entities = HashMap::new();
        entities.insert(1, make_entity(1, CharacterType::Player, &["player"]));

        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);

        let factions = ordered_factions(&entities, 1, &participants);
        assert_eq!(factions.len(), 1);
        assert_eq!(factions[0], "player");
    }

    #[test]
    fn ordered_factions_absent_player_returns_all_participant_keys() {
        let entities: HashMap<i64, Character> = HashMap::new();

        let mut participants = HashMap::new();
        participants.insert("enemy".to_string(), vec![2]);

        let factions = ordered_factions(&entities, 99, &participants);
        assert_eq!(factions, vec!["enemy".to_string()]);
    }

    #[test]
    fn ordered_factions_multiple_non_player_factions_all_included() {
        let mut entities = HashMap::new();
        entities.insert(1, make_entity(1, CharacterType::Player, &["player"]));

        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);
        participants.insert("enemy".to_string(), vec![2]);
        participants.insert("neutral".to_string(), vec![3]);

        let factions = ordered_factions(&entities, 1, &participants);
        assert_eq!(factions[0], "player");
        assert_eq!(factions.len(), 3);
        assert!(factions.contains(&"enemy".to_string()));
        assert!(factions.contains(&"neutral".to_string()));
    }

    // build_participants tests

    #[tokio::test]
    async fn build_participants_player_in_own_faction_bucket() {
        let state = Arc::new(GameState::load(None).unwrap());
        {
            let mut entities = state.active_characters.write().await;
            entities.insert(1, make_entity(1, CharacterType::Player, &["player"]));
            entities.insert(2, make_entity(2, CharacterType::Enemy, &["enemy"]));
        }

        let (_, participants) = build_participants(&state, 1, &[2]).await;
        assert!(participants.get("player").unwrap().contains(&1));
    }

    #[tokio::test]
    async fn build_participants_hostile_in_their_faction_bucket() {
        let state = Arc::new(GameState::load(None).unwrap());
        {
            let mut entities = state.active_characters.write().await;
            entities.insert(1, make_entity(1, CharacterType::Player, &["player"]));
            entities.insert(2, make_entity(2, CharacterType::Enemy, &["enemy"]));
        }

        let (_, participants) = build_participants(&state, 1, &[2]).await;
        assert!(participants.get("enemy").unwrap().contains(&2));
    }

    #[tokio::test]
    async fn build_participants_player_faction_is_first() {
        let state = Arc::new(GameState::load(None).unwrap());
        {
            let mut entities = state.active_characters.write().await;
            entities.insert(1, make_entity(1, CharacterType::Player, &["player"]));
            entities.insert(2, make_entity(2, CharacterType::Enemy, &["enemy"]));
        }

        let (factions, _) = build_participants(&state, 1, &[2]).await;
        assert_eq!(factions[0], "player");
    }

    #[tokio::test]
    async fn build_participants_missing_entity_skipped() {
        let state = Arc::new(GameState::load(None).unwrap());
        {
            let mut entities = state.active_characters.write().await;
            entities.insert(1, make_entity(1, CharacterType::Player, &["player"]));
        }

        let (factions, participants) = build_participants(&state, 1, &[99]).await;
        assert_eq!(factions, vec!["player".to_string()]);
        assert!(participants.get("player").unwrap().contains(&1));
        assert!(!participants.contains_key("enemy"));
    }

    #[tokio::test]
    async fn build_participants_entity_with_multiple_factions_appears_in_each() {
        let state = Arc::new(GameState::load(None).unwrap());
        {
            let mut entities = state.active_characters.write().await;
            entities.insert(1, make_entity(1, CharacterType::Player, &["player"]));
            entities.insert(
                2,
                make_entity(2, CharacterType::Enemy, &["enemy", "bandit"]),
            );
        }

        let (_, participants) = build_participants(&state, 1, &[2]).await;
        assert!(participants.get("enemy").unwrap().contains(&2));
        assert!(participants.get("bandit").unwrap().contains(&2));
    }
}
