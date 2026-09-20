use std::collections::HashMap;
use std::error::Error;

use sqlx::SqlitePool;

use crate::game::RoomFeature;
use crate::persistence::room_feature_repo;

/// Upserts every feature definition in `feature_map` into the database, keeping stored rows
/// in sync with the config files.
pub async fn sync_features_into_db(
    pool: &SqlitePool,
    feature_map: &HashMap<String, RoomFeature>,
) -> Result<(), Box<dyn Error>> {
    for feature in feature_map.values() {
        room_feature_repo::upsert_definition(pool, feature).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::FeatureState;
    use crate::game::component::description::Description;
    use crate::persistence::database::Database;
    use crate::persistence::room_feature_repo;

    fn make_feature(name: &str) -> RoomFeature {
        let mut states = HashMap::new();
        states.insert(
            "closed".to_string(),
            FeatureState {
                description: Description::new(Some("A closed chest.".to_string())),
                items: vec![],
                interact_script: None,
                interact_next_state: None,
            },
        );
        RoomFeature {
            id: "chest".to_string(),
            name: name.to_string(),
            default_state: "closed".to_string(),
            states,
            alt_verbs: vec![],
        }
    }

    #[tokio::test]
    async fn sync_features_into_db_updates_stale_name() {
        let db = Database::connect_in_memory().await.unwrap();

        let stale = make_feature("Oak Chest");
        room_feature_repo::upsert_definition(db.pool(), &stale)
            .await
            .unwrap();

        let mut feature_map = HashMap::new();
        feature_map.insert("chest".to_string(), make_feature("Iron Chest"));

        sync_features_into_db(db.pool(), &feature_map)
            .await
            .unwrap();

        let found = room_feature_repo::find_definition_by_id(db.pool(), "chest")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.name, "Iron Chest");
    }
}
