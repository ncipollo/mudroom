use std::sync::Arc;

use crate::game::component::Location;
use crate::game::config::item_config::select_by_name;
use crate::game::{GameState, RoomFeatureState, WorldLoot};
use crate::persistence::Database;
use crate::persistence::PersistenceError;
use crate::persistence::{room_feature_repo, world_loot_repo};

/// Where a takeable item currently lives.
pub(super) enum TakeSource {
    WorldLoot { loot_id: i64 },
    Feature { room_feature_id: i64 },
}

pub(super) struct TakeMatch {
    pub(super) source: TakeSource,
    pub(super) item_definition_id: String,
    pub(super) name: String,
}

/// A candidate item to take, before name matching has narrowed the field. Feature items
/// and world loot are merged here so ambiguity is resolved across both sources together.
struct TakeCandidate {
    source: TakeSource,
    item_definition_id: String,
}

pub(super) async fn matching_items(
    game_state: &Arc<GameState>,
    db: &Database,
    location: &Location,
    target: &str,
) -> Result<Vec<TakeMatch>, PersistenceError> {
    let loot = world_loot_repo::find_by_location(db.pool(), location).await?;
    let features = room_feature_repo::find_by_location(db.pool(), location).await?;
    let definitions = game_state.item_definitions.read().await;

    let candidates = take_candidates(&loot, &features);
    let selected = select_by_name(candidates, target, |c| {
        definitions.get(&c.item_definition_id)
    });
    Ok(selected
        .into_iter()
        .filter_map(|c| {
            let name = definitions.get(&c.item_definition_id)?.name.clone();
            Some(TakeMatch {
                source: c.source,
                item_definition_id: c.item_definition_id,
                name,
            })
        })
        .collect())
}

fn take_candidates(loot: &[WorldLoot], features: &[RoomFeatureState]) -> Vec<TakeCandidate> {
    let loot_candidates = loot.iter().map(|l| TakeCandidate {
        source: TakeSource::WorldLoot { loot_id: l.id },
        item_definition_id: l.item_definition_id.clone(),
    });
    let feature_candidates = features.iter().flat_map(|feature| {
        feature
            .items
            .iter()
            .map(|item_definition_id| TakeCandidate {
                source: TakeSource::Feature {
                    room_feature_id: feature.id,
                },
                item_definition_id: item_definition_id.clone(),
            })
    });
    loot_candidates.chain(feature_candidates).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::game::component::description::Description;
    use crate::game::component::{EquippedBonuses, ItemDefinition, ItemUseType};
    use crate::game::{Dungeon, FeatureState, Room, RoomFeature, World};
    use crate::persistence::{dungeon_repo, item_repo, room_repo, world_repo};

    fn test_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    fn definition(id: &str, name: &str, alternate_names: &[&str]) -> ItemDefinition {
        ItemDefinition {
            id: id.to_string(),
            name: name.to_string(),
            description: Description::new(None),
            use_type: ItemUseType::Passive,
            item_type: "weapon".to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects: vec![],
            alternate_names: alternate_names.iter().map(|s| s.to_string()).collect(),
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

    async fn game_state_with(defs: Vec<ItemDefinition>, db: &Database) -> Arc<GameState> {
        setup_world(db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        for def in defs {
            seed_loot(&game_state, db, def).await;
        }
        game_state
    }

    fn chest_feature(state: &str, items: Vec<String>) -> RoomFeature {
        let mut states = HashMap::new();
        states.insert(
            state.to_string(),
            FeatureState {
                description: Description::new(None),
                items,
                interact_script: None,
                interact_next_state: None,
            },
        );
        RoomFeature {
            id: "chest".to_string(),
            name: "Oak Chest".to_string(),
            default_state: state.to_string(),
            states,
            alt_verbs: vec![],
        }
    }

    /// Registers a feature holding `def` and places it in `test_location()`.
    async fn seed_feature_item(game_state: &Arc<GameState>, db: &Database, def: ItemDefinition) {
        item_repo::upsert_definition(db.pool(), &def).await.unwrap();
        let feature = chest_feature("open", vec![def.id.clone()]);
        room_feature_repo::upsert_definition(db.pool(), &feature)
            .await
            .unwrap();
        room_feature_repo::insert_placement_if_missing(
            db.pool(),
            &test_location(),
            "chest",
            "open",
            std::slice::from_ref(&def.id),
        )
        .await
        .unwrap();
        game_state
            .item_definitions
            .write()
            .await
            .insert(def.id.clone(), def);
    }

    #[tokio::test]
    async fn matches_primary_name_case_insensitively() {
        let db = Database::connect_in_memory().await.unwrap();
        let game_state =
            game_state_with(vec![definition("spiked_bat", "Spiked Bat", &["bat"])], &db).await;

        let matches = matching_items(&game_state, &db, &test_location(), "spiked bat")
            .await
            .unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Spiked Bat");
    }

    #[tokio::test]
    async fn matches_alternate_name_case_insensitively() {
        let db = Database::connect_in_memory().await.unwrap();
        let game_state =
            game_state_with(vec![definition("spiked_bat", "Spiked Bat", &["bat"])], &db).await;

        let matches = matching_items(&game_state, &db, &test_location(), "BAT")
            .await
            .unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Spiked Bat");
    }

    #[tokio::test]
    async fn ambiguous_when_two_items_share_an_alias() {
        let db = Database::connect_in_memory().await.unwrap();
        let game_state = game_state_with(
            vec![
                definition("spiked_bat", "Spiked Bat", &["stick"]),
                definition("gnarled_club", "Gnarled Club", &["stick"]),
            ],
            &db,
        )
        .await;

        let matches = matching_items(&game_state, &db, &test_location(), "stick")
            .await
            .unwrap();

        assert_eq!(matches.len(), 2);
    }

    #[tokio::test]
    async fn primary_name_match_wins_over_alias_match() {
        let db = Database::connect_in_memory().await.unwrap();
        let game_state = game_state_with(
            vec![
                definition("club", "Club", &[]),
                definition("spiked_bat", "Spiked Bat", &["club"]),
            ],
            &db,
        )
        .await;

        let matches = matching_items(&game_state, &db, &test_location(), "club")
            .await
            .unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Club");
    }

    #[tokio::test]
    async fn matches_item_held_by_a_room_feature() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_world(&db).await;
        let game_state = Arc::new(GameState::load(None).unwrap());
        seed_feature_item(&game_state, &db, definition("torch", "Torch", &[])).await;

        let matches = matching_items(&game_state, &db, &test_location(), "torch")
            .await
            .unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Torch");
    }
}
