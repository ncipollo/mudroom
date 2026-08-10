use sqlx::SqlitePool;

use crate::game::component::description::Description;
use crate::game::component::{EquippedBonuses, ItemDefinition, ItemUseType, UseEffect};
use crate::persistence::error::PersistenceError;

type ItemDefinitionRow = (
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    String,
);

pub async fn upsert_definition(
    pool: &SqlitePool,
    def: &ItemDefinition,
) -> Result<(), PersistenceError> {
    let use_type_json = serde_json::to_string(&def.use_type).unwrap_or_default();
    let attribute_bonuses_json =
        serde_json::to_string(&def.equipped_bonuses.attributes).unwrap_or_default();
    let use_effects_json = serde_json::to_string(&def.use_effects).unwrap_or_default();
    let equipped_abilities_json =
        serde_json::to_string(&def.equipped_bonuses.equipped).unwrap_or_default();
    sqlx::query(
        "INSERT INTO item_definitions \
         (id, name, description, use_type, item_type, attribute_bonuses_json, use_effects_json, equipped_abilities_json) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(id) DO UPDATE SET \
             name = excluded.name, \
             description = excluded.description, \
             use_type = excluded.use_type, \
             item_type = excluded.item_type, \
             attribute_bonuses_json = excluded.attribute_bonuses_json, \
             use_effects_json = excluded.use_effects_json, \
             equipped_abilities_json = excluded.equipped_abilities_json",
    )
    .bind(&def.id)
    .bind(&def.name)
    .bind(&def.description.text)
    .bind(&use_type_json)
    .bind(&def.item_type)
    .bind(&attribute_bonuses_json)
    .bind(&use_effects_json)
    .bind(&equipped_abilities_json)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_all_definitions(
    pool: &SqlitePool,
) -> Result<Vec<ItemDefinition>, PersistenceError> {
    let rows: Vec<ItemDefinitionRow> = sqlx::query_as(
        "SELECT id, name, description, use_type, item_type, attribute_bonuses_json, \
             use_effects_json, equipped_abilities_json FROM item_definitions",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                name,
                description,
                use_type_json,
                item_type,
                attribute_bonuses_json,
                use_effects_json,
                equipped_abilities_json,
            )| {
                let use_type: ItemUseType =
                    serde_json::from_str(&use_type_json).unwrap_or_else(|e| {
                        tracing::warn!("Failed to deserialize use_type for item {id}: {e}");
                        ItemUseType::Used
                    });
                let attribute_bonuses = serde_json::from_str(&attribute_bonuses_json)
                    .unwrap_or_else(|e| {
                        tracing::warn!(
                            "Failed to deserialize attribute_bonuses for item {id}: {e}"
                        );
                        vec![]
                    });
                let use_effects: Vec<UseEffect> = serde_json::from_str(&use_effects_json)
                    .unwrap_or_else(|e| {
                        tracing::warn!("Failed to deserialize use_effects for item {id}: {e}");
                        vec![]
                    });
                let equipped_abilities = serde_json::from_str(&equipped_abilities_json)
                    .unwrap_or_else(|e| {
                        tracing::warn!(
                            "Failed to deserialize equipped_abilities for item {id}: {e}"
                        );
                        vec![]
                    });
                ItemDefinition {
                    id,
                    name,
                    description: Description::new(description),
                    use_type,
                    item_type,
                    equipped_bonuses: EquippedBonuses {
                        attributes: attribute_bonuses,
                        equipped: equipped_abilities,
                    },
                    use_effects,
                }
            },
        )
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::AttributeBonus;
    use crate::persistence::database::Database;

    fn health_tonic() -> ItemDefinition {
        ItemDefinition {
            id: "health_tonic".to_string(),
            name: "Health Tonic".to_string(),
            description: Description::new(Some("A restorative brew.".to_string())),
            use_type: ItemUseType::Used,
            item_type: "medicine".to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects: vec![],
        }
    }

    fn leather_vest() -> ItemDefinition {
        ItemDefinition {
            id: "leather_vest".to_string(),
            name: "Leather Vest".to_string(),
            description: Description::new(Some("A simple protective vest.".to_string())),
            use_type: ItemUseType::Passive,
            item_type: "armor".to_string(),
            equipped_bonuses: EquippedBonuses {
                attributes: vec![AttributeBonus {
                    attribute_id: "defense".to_string(),
                    amount: 5,
                }],
                equipped: vec![],
            },
            use_effects: vec![],
        }
    }

    #[tokio::test]
    async fn upsert_and_find_all_definitions() {
        let db = Database::connect_in_memory().await.unwrap();
        upsert_definition(db.pool(), &health_tonic()).await.unwrap();
        upsert_definition(db.pool(), &leather_vest()).await.unwrap();

        let defs = find_all_definitions(db.pool()).await.unwrap();
        assert_eq!(defs.len(), 2);
        let vest = defs.iter().find(|d| d.id == "leather_vest").unwrap();
        assert_eq!(vest.use_type, ItemUseType::Passive);
        assert_eq!(vest.equipped_bonuses.attributes.len(), 1);
    }

    #[tokio::test]
    async fn upsert_definition_is_idempotent() {
        let db = Database::connect_in_memory().await.unwrap();
        upsert_definition(db.pool(), &health_tonic()).await.unwrap();
        upsert_definition(db.pool(), &health_tonic()).await.unwrap();

        let defs = find_all_definitions(db.pool()).await.unwrap();
        assert_eq!(defs.len(), 1);
    }

    #[tokio::test]
    async fn upsert_definition_updates_existing() {
        let db = Database::connect_in_memory().await.unwrap();
        upsert_definition(db.pool(), &health_tonic()).await.unwrap();

        let mut updated = health_tonic();
        updated.name = "Greater Health Tonic".to_string();
        upsert_definition(db.pool(), &updated).await.unwrap();

        let defs = find_all_definitions(db.pool()).await.unwrap();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "Greater Health Tonic");
    }
}
