use std::collections::HashMap;
use std::error::Error;

use sqlx::SqlitePool;

use crate::game::{Location, Room, RoomFeature, Universe};
use crate::persistence::room_feature_repo;

/// Syncs every room's configured feature placements into the database. When `reset` is
/// true, placements that already exist are forced back to their feature's default state and
/// items (see [`room_feature_repo::reset_placement`]) — used for an operator-triggered
/// feature reset, not a normal reload.
pub async fn load_feature_placements_into_db(
    pool: &SqlitePool,
    universe: &Universe,
    feature_map: &HashMap<String, RoomFeature>,
    reset: bool,
) -> Result<(), Box<dyn Error>> {
    for world in universe.worlds.values() {
        for dungeon in world.dungeons.values() {
            for room in dungeon.rooms.values() {
                sync_room_features(pool, &world.id, &dungeon.id, room, feature_map, reset).await?;
            }
        }
    }
    Ok(())
}

async fn sync_room_features(
    pool: &SqlitePool,
    world_id: &str,
    dungeon_id: &str,
    room: &Room,
    feature_map: &HashMap<String, RoomFeature>,
    reset: bool,
) -> Result<(), Box<dyn Error>> {
    for feature_id in &room.features {
        if let Some(feature) = feature_map.get(feature_id) {
            let location = Location {
                world_id: world_id.to_string(),
                dungeon_id: dungeon_id.to_string(),
                room_id: room.id.clone(),
            };
            let items = feature
                .default_state()
                .map(|state| state.items.clone())
                .unwrap_or_default();
            room_feature_repo::insert_placement_if_missing(
                pool,
                &location,
                feature_id,
                &feature.default_state,
                &items,
            )
            .await?;
            if reset {
                room_feature_repo::reset_placement(
                    pool,
                    &location,
                    feature_id,
                    &feature.default_state,
                    &items,
                )
                .await?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::description::Description;
    use crate::game::{Dungeon, FeatureState, World};
    use crate::persistence::database::Database;
    use crate::persistence::room_feature_repo;

    use super::super::universe_sync::load_map_into_db;

    fn chest_feature() -> RoomFeature {
        let mut states = HashMap::new();
        states.insert(
            "closed".to_string(),
            FeatureState {
                description: Description::new(Some("A closed chest.".to_string())),
                items: vec![],
                interact_script: None,
                interact_next_state: Some("open".to_string()),
                alt_verbs: vec![],
            },
        );
        states.insert(
            "open".to_string(),
            FeatureState {
                description: Description::new(Some("An open chest.".to_string())),
                items: vec!["medicine".to_string()],
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

    fn feature_map() -> HashMap<String, RoomFeature> {
        let mut map = HashMap::new();
        map.insert("chest".to_string(), chest_feature());
        map
    }

    fn make_universe_with_feature() -> Universe {
        let mut universe = Universe::default();
        let mut world = World::new("w1".to_string());
        let mut dungeon = Dungeon::new("d1".to_string());
        let mut room = Room::new(
            "r1".to_string(),
            Description::new(Some("A room.".to_string())),
        );
        room.features.push("chest".to_string());
        dungeon.rooms.insert("r1".to_string(), room);
        world.dungeons.insert("d1".to_string(), dungeon);
        universe.worlds.insert("w1".to_string(), world);
        universe
    }

    fn chest_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    async fn setup_feature_definition(db: &Database) {
        room_feature_repo::upsert_definition(db.pool(), &chest_feature())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn load_feature_placements_into_db_seeds_room_features() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_feature_definition(&db).await;
        let universe = make_universe_with_feature();
        load_map_into_db(db.pool(), &universe).await.unwrap();

        load_feature_placements_into_db(db.pool(), &universe, &feature_map(), false)
            .await
            .unwrap();

        let placed = room_feature_repo::find_by_location(db.pool(), &chest_location())
            .await
            .unwrap();
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].feature_definition_id, "chest");
        assert_eq!(placed[0].current_state, "closed");
        assert!(placed[0].items.is_empty());
    }

    #[tokio::test]
    async fn load_feature_placements_into_db_seeds_items_from_default_state() {
        let db = Database::connect_in_memory().await.unwrap();
        let mut feature = chest_feature();
        feature.default_state = "open".to_string();
        let mut feature_map = HashMap::new();
        feature_map.insert("chest".to_string(), feature.clone());
        room_feature_repo::upsert_definition(db.pool(), &feature)
            .await
            .unwrap();
        let universe = make_universe_with_feature();
        load_map_into_db(db.pool(), &universe).await.unwrap();

        load_feature_placements_into_db(db.pool(), &universe, &feature_map, false)
            .await
            .unwrap();

        let placed = room_feature_repo::find_by_location(db.pool(), &chest_location())
            .await
            .unwrap();
        assert_eq!(placed[0].items, vec!["medicine".to_string()]);
    }

    #[tokio::test]
    async fn load_feature_placements_into_db_is_idempotent() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_feature_definition(&db).await;
        let universe = make_universe_with_feature();
        load_map_into_db(db.pool(), &universe).await.unwrap();

        load_feature_placements_into_db(db.pool(), &universe, &feature_map(), false)
            .await
            .unwrap();
        load_feature_placements_into_db(db.pool(), &universe, &feature_map(), false)
            .await
            .unwrap();

        let placed = room_feature_repo::find_by_location(db.pool(), &chest_location())
            .await
            .unwrap();
        assert_eq!(placed.len(), 1);
    }

    #[tokio::test]
    async fn load_feature_placements_into_db_preserves_modified_state_on_resync() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_feature_definition(&db).await;
        let universe = make_universe_with_feature();
        load_map_into_db(db.pool(), &universe).await.unwrap();

        load_feature_placements_into_db(db.pool(), &universe, &feature_map(), false)
            .await
            .unwrap();
        let placed = room_feature_repo::find_by_location(db.pool(), &chest_location())
            .await
            .unwrap();
        room_feature_repo::update_state(db.pool(), placed[0].id, "open")
            .await
            .unwrap();

        load_feature_placements_into_db(db.pool(), &universe, &feature_map(), false)
            .await
            .unwrap();

        let placed = room_feature_repo::find_by_location(db.pool(), &chest_location())
            .await
            .unwrap();
        assert_eq!(placed[0].current_state, "open");
    }

    #[tokio::test]
    async fn load_feature_placements_into_db_skips_unknown_feature_ids() {
        let db = Database::connect_in_memory().await.unwrap();
        let universe = make_universe_with_feature();
        load_map_into_db(db.pool(), &universe).await.unwrap();

        load_feature_placements_into_db(db.pool(), &universe, &HashMap::new(), false)
            .await
            .unwrap();

        let placed = room_feature_repo::find_by_location(db.pool(), &chest_location())
            .await
            .unwrap();
        assert!(placed.is_empty());
    }

    #[tokio::test]
    async fn load_feature_placements_into_db_with_reset_overwrites_modified_state_and_items() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_feature_definition(&db).await;
        let universe = make_universe_with_feature();
        load_map_into_db(db.pool(), &universe).await.unwrap();

        load_feature_placements_into_db(db.pool(), &universe, &feature_map(), false)
            .await
            .unwrap();
        let placed = room_feature_repo::find_by_location(db.pool(), &chest_location())
            .await
            .unwrap();
        room_feature_repo::update_state(db.pool(), placed[0].id, "open")
            .await
            .unwrap();

        load_feature_placements_into_db(db.pool(), &universe, &feature_map(), true)
            .await
            .unwrap();

        let placed = room_feature_repo::find_by_location(db.pool(), &chest_location())
            .await
            .unwrap();
        assert_eq!(placed[0].current_state, "closed");
        assert!(placed[0].items.is_empty());
    }

    #[tokio::test]
    async fn load_feature_placements_into_db_with_reset_still_seeds_new_placements() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_feature_definition(&db).await;
        let universe = make_universe_with_feature();
        load_map_into_db(db.pool(), &universe).await.unwrap();

        load_feature_placements_into_db(db.pool(), &universe, &feature_map(), true)
            .await
            .unwrap();

        let placed = room_feature_repo::find_by_location(db.pool(), &chest_location())
            .await
            .unwrap();
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].current_state, "closed");
    }
}
