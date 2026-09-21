use std::sync::Arc;

use crate::game::GameState;
use crate::game::component::{Description, ItemDefinition, Location};
use crate::game::config::item_config;
use crate::game::map::universe::room_feature;
use crate::persistence::Database;
use crate::persistence::PersistenceError;
use crate::persistence::{room_feature_repo, world_loot_repo};

/// A look-at target that matched the player's search, merged across items and room
/// features so ambiguity is resolved across both sources together.
pub(super) struct LookMatch {
    pub(super) name: String,
    pub(super) description: Description,
}

pub(super) async fn matching_targets(
    game_state: &Arc<GameState>,
    db: &Database,
    location: &Location,
    target: &str,
) -> Result<Vec<LookMatch>, PersistenceError> {
    let mut matches = matching_items(game_state, db, location, target).await?;
    matches.extend(matching_features(db, location, target).await?);
    Ok(matches)
}

async fn matching_items(
    game_state: &Arc<GameState>,
    db: &Database,
    location: &Location,
    target: &str,
) -> Result<Vec<LookMatch>, PersistenceError> {
    let loot = world_loot_repo::find_by_location(db.pool(), location).await?;
    let definitions = game_state.item_definitions.read().await;
    let selected: Vec<&ItemDefinition> = item_config::select_by_name(loot.iter(), target, |l| {
        definitions.get(&l.item_definition_id)
    })
    .into_iter()
    .filter_map(|l| definitions.get(&l.item_definition_id))
    .collect();

    Ok(selected
        .into_iter()
        .map(|def| LookMatch {
            name: def.name.clone(),
            description: def.description.clone(),
        })
        .collect())
}

async fn matching_features(
    db: &Database,
    location: &Location,
    target: &str,
) -> Result<Vec<LookMatch>, PersistenceError> {
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

    Ok(room_feature::select_by_name(candidates, target, |c| &c.1)
        .into_iter()
        .filter_map(|(placement, def)| {
            let state = def.states.get(&placement.current_state)?;
            Some(LookMatch {
                name: def.name.clone(),
                description: state.description.clone(),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::game::component::{EquippedBonuses, ItemUseType};
    use crate::game::{Dungeon, FeatureState, Room, RoomFeature, World};
    use crate::persistence::{dungeon_repo, item_repo, room_repo, world_repo};

    fn test_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    fn definition(id: &str, name: &str) -> ItemDefinition {
        ItemDefinition {
            id: id.to_string(),
            name: name.to_string(),
            description: Description::new(Some(format!("A {name} description."))),
            use_type: ItemUseType::Passive,
            item_type: "weapon".to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects: vec![],
            alternate_names: vec![],
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

    async fn seed_loot(game_state: &Arc<GameState>, db: &Database, def: ItemDefinition) {
        item_repo::upsert_definition(db.pool(), &def).await.unwrap();
        world_loot_repo::insert_config_loot_if_missing(db.pool(), &test_location(), &def.id)
            .await
            .unwrap();
        game_state
            .item_definitions
            .write()
            .await
            .insert(def.id.clone(), def);
    }

    fn chest_feature(states: Vec<(&str, &str)>, default_state: &str) -> RoomFeature {
        let mut state_map = HashMap::new();
        for (state, description) in states {
            state_map.insert(
                state.to_string(),
                FeatureState {
                    description: Description::new(Some(description.to_string())),
                    items: vec![],
                    interact_script: None,
                    interact_next_state: None,
                    alt_verbs: vec![],
                },
            );
        }
        RoomFeature {
            id: "chest".to_string(),
            name: "Oak Chest".to_string(),
            default_state: default_state.to_string(),
            states: state_map,
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
    async fn matches_item_by_name() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        seed_loot(&game_state, &db, definition("torch", "Torch")).await;

        let matches = matching_targets(&game_state, &db, &test_location(), "torch")
            .await
            .unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Torch");
    }

    #[tokio::test]
    async fn matches_feature_current_state_description() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        let feature = chest_feature(
            vec![
                ("closed", "A closed oak chest."),
                ("open", "An open oak chest."),
            ],
            "closed",
        );
        seed_feature(&db, &feature, "open").await;

        let matches = matching_targets(&game_state, &db, &test_location(), "oak chest")
            .await
            .unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Oak Chest");
        assert_eq!(
            matches[0].description.text.as_deref(),
            Some("An open oak chest.")
        );
    }

    #[tokio::test]
    async fn matches_feature_closed_state_description() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        let feature = chest_feature(
            vec![
                ("closed", "A closed oak chest."),
                ("open", "An open oak chest."),
            ],
            "closed",
        );
        seed_feature(&db, &feature, "closed").await;

        let matches = matching_targets(&game_state, &db, &test_location(), "oak chest")
            .await
            .unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].description.text.as_deref(),
            Some("A closed oak chest.")
        );
    }

    #[tokio::test]
    async fn matches_feature_alternate_name_case_insensitively() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        let mut feature = chest_feature(vec![("closed", "A closed oak chest.")], "closed");
        feature.alternate_names = vec!["chest".to_string()];
        seed_feature(&db, &feature, "closed").await;

        let matches = matching_targets(&game_state, &db, &test_location(), "CHEST")
            .await
            .unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Oak Chest");
    }

    #[tokio::test]
    async fn feature_primary_name_match_wins_over_alias_match() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        let mut club = chest_feature(vec![("closed", "A wooden club.")], "closed");
        club.id = "club".to_string();
        club.name = "Club".to_string();
        seed_feature(&db, &club, "closed").await;
        let mut spiked_bat = chest_feature(vec![("closed", "A spiked bat.")], "closed");
        spiked_bat.id = "spiked_bat".to_string();
        spiked_bat.name = "Spiked Bat".to_string();
        spiked_bat.alternate_names = vec!["club".to_string()];
        seed_feature(&db, &spiked_bat, "closed").await;

        let matches = matching_targets(&game_state, &db, &test_location(), "club")
            .await
            .unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Club");
    }

    #[tokio::test]
    async fn merges_item_and_feature_matches_for_a_name_collision() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        seed_loot(&game_state, &db, definition("sword", "Sword")).await;
        let feature = chest_feature(vec![("closed", "A locked cabinet.")], "closed");
        let mut feature = feature;
        feature.name = "Sword".to_string();
        seed_feature(&db, &feature, "closed").await;

        let matches = matching_targets(&game_state, &db, &test_location(), "sword")
            .await
            .unwrap();

        assert_eq!(matches.len(), 2);
    }

    #[tokio::test]
    async fn does_not_match_feature_in_a_different_room() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        let feature = chest_feature(vec![("closed", "A closed oak chest.")], "closed");
        room_feature_repo::upsert_definition(db.pool(), &feature)
            .await
            .unwrap();
        let other_room = Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r2".to_string(),
        };
        room_repo::insert(
            db.pool(),
            &Room::new("r2".to_string(), Description::new(None)),
            "d1",
        )
        .await
        .unwrap();
        room_feature_repo::insert_placement_if_missing(
            db.pool(),
            &other_room,
            &feature.id,
            "closed",
            &[],
        )
        .await
        .unwrap();

        let matches = matching_targets(&game_state, &db, &test_location(), "oak chest")
            .await
            .unwrap();

        assert!(matches.is_empty());
    }
}
