use crate::game::component::Location;
use crate::persistence::Database;
use crate::persistence::PersistenceError;
use crate::persistence::room_feature_repo;

/// A room feature that matched the player's `interact`/alt-verb target.
pub(super) struct InteractMatch {
    pub(super) room_feature_id: i64,
    pub(super) name: String,
    pub(super) next_state: Option<String>,
}

pub(super) async fn matching_features(
    db: &Database,
    location: &Location,
    verb: &str,
    target: &str,
) -> Result<Vec<InteractMatch>, PersistenceError> {
    let placements = room_feature_repo::find_by_location(db.pool(), location).await?;
    let mut matches = Vec::new();
    for placement in placements {
        let Some(def) =
            room_feature_repo::find_definition_by_id(db.pool(), &placement.feature_definition_id)
                .await?
        else {
            continue;
        };
        if !def.name.eq_ignore_ascii_case(target) || !def.allows_verb(verb) {
            continue;
        }
        let next_state = def
            .states
            .get(&placement.current_state)
            .and_then(|state| state.interact_next_state.clone());
        matches.push(InteractMatch {
            room_feature_id: placement.id,
            name: def.name,
            next_state,
        });
    }
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::game::component::Description;
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

    fn chest_feature(alt_verbs: Vec<&str>) -> RoomFeature {
        let mut states = HashMap::new();
        states.insert(
            "closed".to_string(),
            FeatureState {
                description: Description::new(Some("A closed chest.".to_string())),
                items: vec![],
                interact_script: None,
                interact_next_state: Some("open".to_string()),
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

    async fn seed_feature(db: &Database, feature: &RoomFeature, current_state: &str) {
        room_feature_repo::upsert_definition(db.pool(), feature)
            .await
            .unwrap();
        room_feature_repo::insert_placement_if_missing(
            db.pool(),
            &test_location(),
            &feature.id,
            current_state,
            &[],
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn matches_by_name_with_the_generic_interact_verb() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        seed_feature(&db, &chest_feature(vec![]), "closed").await;

        let matches = matching_features(&db, &test_location(), "interact", "oak chest")
            .await
            .unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].next_state.as_deref(), Some("open"));
    }

    #[tokio::test]
    async fn matches_by_name_with_a_declared_alt_verb() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        seed_feature(&db, &chest_feature(vec!["open"]), "closed").await;

        let matches = matching_features(&db, &test_location(), "OPEN", "oak chest")
            .await
            .unwrap();

        assert_eq!(matches.len(), 1);
    }

    #[tokio::test]
    async fn does_not_match_an_undeclared_alt_verb() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        seed_feature(&db, &chest_feature(vec![]), "closed").await;

        let matches = matching_features(&db, &test_location(), "push", "oak chest")
            .await
            .unwrap();

        assert!(matches.is_empty());
    }

    #[tokio::test]
    async fn resolves_next_state_from_the_current_state() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        seed_feature(&db, &chest_feature(vec![]), "open").await;

        let matches = matching_features(&db, &test_location(), "interact", "oak chest")
            .await
            .unwrap();

        assert_eq!(matches.len(), 1);
        assert!(matches[0].next_state.is_none());
    }
}
