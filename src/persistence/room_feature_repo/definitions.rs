use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::game::{FeatureState, RoomFeature};
use crate::persistence::error::PersistenceError;

type FeatureDefinitionRow = (String, String, String, String, String);

pub async fn upsert_definition(
    pool: &SqlitePool,
    feature: &RoomFeature,
) -> Result<(), PersistenceError> {
    let states_json = serde_json::to_string(&feature.states)?;
    let alt_verbs_json = serde_json::to_string(&feature.alt_verbs)?;
    sqlx::query(
        "INSERT INTO feature_definitions (id, name, default_state, states_json, alt_verbs_json) \
         VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT(id) DO UPDATE SET \
             name = excluded.name, \
             default_state = excluded.default_state, \
             states_json = excluded.states_json, \
             alt_verbs_json = excluded.alt_verbs_json",
    )
    .bind(&feature.id)
    .bind(&feature.name)
    .bind(&feature.default_state)
    .bind(&states_json)
    .bind(&alt_verbs_json)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_definition_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<RoomFeature>, PersistenceError> {
    let row: Option<FeatureDefinitionRow> = sqlx::query_as(
        "SELECT id, name, default_state, states_json, alt_verbs_json \
         FROM feature_definitions WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(parse_feature_definition).transpose()
}

fn parse_feature_definition(row: FeatureDefinitionRow) -> Result<RoomFeature, PersistenceError> {
    let (id, name, default_state, states_json, alt_verbs_json) = row;
    let states: HashMap<String, FeatureState> = serde_json::from_str(&states_json)?;
    let alt_verbs: Vec<String> = serde_json::from_str(&alt_verbs_json)?;
    Ok(RoomFeature {
        id,
        name,
        default_state,
        states,
        alt_verbs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::description::Description;
    use crate::persistence::database::Database;

    fn chest_feature() -> RoomFeature {
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
                items: vec!["medicine".to_string()],
                interact_script: None,
                interact_next_state: None,
            },
        );
        RoomFeature {
            id: "chest".to_string(),
            name: "Oak Chest".to_string(),
            default_state: "closed".to_string(),
            states,
            alt_verbs: vec!["open".to_string()],
        }
    }

    #[tokio::test]
    async fn upsert_definition_and_find_by_id_round_trip() {
        let db = Database::connect_in_memory().await.unwrap();
        let feature = chest_feature();
        upsert_definition(db.pool(), &feature).await.unwrap();

        let found = find_definition_by_id(db.pool(), "chest")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found, feature);
    }

    #[tokio::test]
    async fn upsert_definition_updates_existing_row() {
        let db = Database::connect_in_memory().await.unwrap();
        let mut feature = chest_feature();
        upsert_definition(db.pool(), &feature).await.unwrap();

        feature.name = "Iron Chest".to_string();
        upsert_definition(db.pool(), &feature).await.unwrap();

        let found = find_definition_by_id(db.pool(), "chest")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.name, "Iron Chest");
    }

    #[tokio::test]
    async fn find_definition_by_id_returns_none_for_missing() {
        let db = Database::connect_in_memory().await.unwrap();
        let found = find_definition_by_id(db.pool(), "missing").await.unwrap();
        assert!(found.is_none());
    }
}
