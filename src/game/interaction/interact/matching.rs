use crate::game::RoomFeatureState;
use crate::game::component::Location;
use crate::game::map::universe::room_feature;
use crate::persistence::Database;
use crate::persistence::PersistenceError;
use crate::persistence::room_feature_repo;

/// A room feature that matched the player's `interact`/alt-verb target.
pub(super) struct InteractMatch {
    pub(super) room_feature_id: i64,
    pub(super) name: String,
    pub(super) next_state: Option<String>,
}

/// The result of matching a player's target against room features at their location.
/// `matches` is empty when either nothing matched by name, or something matched by name
/// but `verb` doesn't apply to its current state — `verb_mismatches` (feature names) tells
/// those two cases apart so the caller can give clearer feedback than a silent no-op.
pub(super) struct MatchOutcome {
    pub(super) matches: Vec<InteractMatch>,
    pub(super) verb_mismatches: Vec<String>,
}

pub(super) async fn matching_features(
    db: &Database,
    location: &Location,
    verb: &str,
    target: &str,
) -> Result<MatchOutcome, PersistenceError> {
    let placements = room_feature_repo::find_by_location(db.pool(), location).await?;
    let mut candidates = Vec::new();
    for placement in placements {
        let Some(def) =
            room_feature_repo::find_definition_by_id(db.pool(), &placement.feature_definition_id)
                .await?
        else {
            continue;
        };
        candidates.push((placement, def));
    }

    let name_matches = room_feature::select_by_name(candidates, target, |c| &c.1);
    Ok(split_by_verb(name_matches, verb))
}

/// Splits name-matched candidates into those whose current state allows `verb` (as
/// [`InteractMatch`]es) and those that matched by name but not by verb (by name, for
/// feedback).
fn split_by_verb(
    name_matches: Vec<(RoomFeatureState, room_feature::RoomFeature)>,
    verb: &str,
) -> MatchOutcome {
    let mut matches = Vec::new();
    let mut verb_mismatches = Vec::new();
    for (placement, def) in name_matches {
        let Some(state) = def.states.get(&placement.current_state) else {
            continue;
        };
        if state.allows_verb(verb) {
            matches.push(InteractMatch {
                room_feature_id: placement.id,
                name: def.name,
                next_state: state.interact_next_state.clone(),
            });
        } else {
            verb_mismatches.push(def.name);
        }
    }
    MatchOutcome {
        matches,
        verb_mismatches,
    }
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
                alt_verbs: alt_verbs.into_iter().map(str::to_string).collect(),
            },
        );
        states.insert(
            "open".to_string(),
            FeatureState {
                description: Description::new(Some("An open chest.".to_string())),
                items: vec![],
                interact_script: None,
                interact_next_state: None,
                alt_verbs: vec![],
            },
        );
        RoomFeature {
            id: "chest".to_string(),
            name: "Oak Chest".to_string(),
            default_state: "closed".to_string(),
            states,
            alternate_names: vec![],
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

        let outcome = matching_features(&db, &test_location(), "interact", "oak chest")
            .await
            .unwrap();

        assert_eq!(outcome.matches.len(), 1);
        assert_eq!(outcome.matches[0].next_state.as_deref(), Some("open"));
    }

    #[tokio::test]
    async fn matches_by_name_with_a_declared_alt_verb() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        seed_feature(&db, &chest_feature(vec!["open"]), "closed").await;

        let outcome = matching_features(&db, &test_location(), "OPEN", "oak chest")
            .await
            .unwrap();

        assert_eq!(outcome.matches.len(), 1);
    }

    #[tokio::test]
    async fn undeclared_alt_verb_is_reported_as_a_verb_mismatch() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        seed_feature(&db, &chest_feature(vec![]), "closed").await;

        let outcome = matching_features(&db, &test_location(), "push", "oak chest")
            .await
            .unwrap();

        assert!(outcome.matches.is_empty());
        assert_eq!(outcome.verb_mismatches, vec!["Oak Chest".to_string()]);
    }

    #[tokio::test]
    async fn no_name_match_reports_no_verb_mismatch_either() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;

        let outcome = matching_features(&db, &test_location(), "interact", "oak chest")
            .await
            .unwrap();

        assert!(outcome.matches.is_empty());
        assert!(outcome.verb_mismatches.is_empty());
    }

    #[tokio::test]
    async fn matches_by_name_but_not_by_verb_reports_a_verb_mismatch() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        seed_feature(&db, &chest_feature(vec!["open"]), "open").await;

        let outcome = matching_features(&db, &test_location(), "open", "oak chest")
            .await
            .unwrap();

        assert!(outcome.matches.is_empty());
        assert_eq!(outcome.verb_mismatches, vec!["Oak Chest".to_string()]);
    }

    #[tokio::test]
    async fn resolves_next_state_from_the_current_state() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        seed_feature(&db, &chest_feature(vec![]), "open").await;

        let outcome = matching_features(&db, &test_location(), "interact", "oak chest")
            .await
            .unwrap();

        assert_eq!(outcome.matches.len(), 1);
        assert!(outcome.matches[0].next_state.is_none());
    }

    #[tokio::test]
    async fn matches_by_alternate_name_case_insensitively() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let mut feature = chest_feature(vec![]);
        feature.alternate_names = vec!["chest".to_string()];
        seed_feature(&db, &feature, "closed").await;

        let outcome = matching_features(&db, &test_location(), "interact", "CHEST")
            .await
            .unwrap();

        assert_eq!(outcome.matches.len(), 1);
    }

    #[tokio::test]
    async fn primary_name_match_wins_over_alias_match() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let mut club = chest_feature(vec![]);
        club.id = "club".to_string();
        club.name = "Club".to_string();
        seed_feature(&db, &club, "closed").await;
        let mut spiked_bat = chest_feature(vec![]);
        spiked_bat.id = "spiked_bat".to_string();
        spiked_bat.name = "Spiked Bat".to_string();
        spiked_bat.alternate_names = vec!["club".to_string()];
        seed_feature(&db, &spiked_bat, "closed").await;

        let outcome = matching_features(&db, &test_location(), "interact", "club")
            .await
            .unwrap();

        assert_eq!(outcome.matches.len(), 1);
        assert_eq!(outcome.matches[0].name, "Club");
    }
}
