use sqlx::SqlitePool;

use crate::game::component::{Ability, AbilityRole, AbilityTargetType};
use crate::persistence::error::PersistenceError;

type AbilityRow = (
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    String,
    String,
);

pub async fn upsert(pool: &SqlitePool, ability: &Ability) -> Result<(), PersistenceError> {
    let effects_json = serde_json::to_string(&ability.effects).unwrap_or_default();
    let costs_json = serde_json::to_string(&ability.costs).unwrap_or_default();
    let modifiers_json = serde_json::to_string(&ability.modifiers).unwrap_or_default();
    let engagement_types_json =
        serde_json::to_string(&ability.engagement_types).unwrap_or_default();
    let targets_json = serde_json::to_string(&ability.targets).unwrap_or_default();
    let role_str = ability_role_to_str(&ability.role);
    sqlx::query(
        "INSERT OR REPLACE INTO abilities \
         (id, name, description, effects_json, costs_json, modifiers_json, engagement_types_json, role, targets_json) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&ability.id)
    .bind(&ability.name)
    .bind(&ability.description)
    .bind(&effects_json)
    .bind(&costs_json)
    .bind(&modifiers_json)
    .bind(&engagement_types_json)
    .bind(role_str)
    .bind(&targets_json)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_by_entity(
    pool: &SqlitePool,
    entity_id: i64,
) -> Result<Vec<Ability>, PersistenceError> {
    let rows: Vec<AbilityRow> = sqlx::query_as(
        "SELECT a.id, a.name, a.description, a.effects_json, a.costs_json, \
             a.modifiers_json, a.engagement_types_json, a.role, a.targets_json \
             FROM abilities a \
             JOIN entity_abilities ea ON a.id = ea.ability_id \
             WHERE ea.entity_id = ?",
    )
    .bind(entity_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                name,
                description,
                effects_json,
                costs_json,
                modifiers_json,
                et_json,
                role_str,
                targets_json,
            )| {
                let effects = serde_json::from_str(&effects_json).unwrap_or_else(|e| {
                    tracing::warn!("Failed to deserialize effects for ability {id}: {e}");
                    vec![]
                });
                let costs = serde_json::from_str(&costs_json).unwrap_or_else(|e| {
                    tracing::warn!("Failed to deserialize costs for ability {id}: {e}");
                    vec![]
                });
                let modifiers = serde_json::from_str(&modifiers_json).unwrap_or_else(|e| {
                    tracing::warn!("Failed to deserialize modifiers for ability {id}: {e}");
                    vec![]
                });
                let engagement_types = serde_json::from_str(&et_json).unwrap_or_else(|e| {
                    tracing::warn!("Failed to deserialize engagement_types for ability {id}: {e}");
                    vec![]
                });
                let targets: Vec<AbilityTargetType> = serde_json::from_str(&targets_json)
                    .unwrap_or_else(|e| {
                        tracing::warn!("Failed to deserialize targets for ability {id}: {e}");
                        vec![]
                    });
                Ability {
                    id,
                    name,
                    description,
                    effects,
                    costs,
                    modifiers,
                    engagement_types,
                    role: ability_role_from_str(&role_str),
                    targets,
                }
            },
        )
        .collect())
}

pub async fn set_entity_abilities(
    pool: &SqlitePool,
    entity_id: i64,
    ability_ids: &[&str],
) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM entity_abilities WHERE entity_id = ?")
        .bind(entity_id)
        .execute(pool)
        .await?;
    for ability_id in ability_ids {
        sqlx::query("INSERT INTO entity_abilities (entity_id, ability_id) VALUES (?, ?)")
            .bind(entity_id)
            .bind(ability_id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

fn ability_role_to_str(role: &AbilityRole) -> &'static str {
    match role {
        AbilityRole::Attack => "attack",
        AbilityRole::Defend => "defend",
        AbilityRole::World => "world",
    }
}

fn ability_role_from_str(s: &str) -> AbilityRole {
    match s {
        "defend" => AbilityRole::Defend,
        "world" => AbilityRole::World,
        _ => AbilityRole::Attack,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::effect::{
        Effect, EffectDescription, EffectScope, EffectType, TriggerInfo,
    };
    use crate::game::engagement::EngagementType;
    use crate::game::{Description, Dungeon, Entity, EntityType, Location, Room, World};
    use crate::persistence::database::Database;
    use crate::persistence::{dungeon_repo, entity_repo, room_repo, world_repo};

    async fn setup(db: &Database) -> i64 {
        let world = World::new("w1".to_string());
        world_repo::insert(db.pool(), &world).await.unwrap();
        let dungeon = Dungeon::new("d1".to_string());
        dungeon_repo::insert(db.pool(), &dungeon, "w1")
            .await
            .unwrap();
        let room = Room::new("r1".to_string(), Description::new(None));
        room_repo::insert(db.pool(), &room, "d1").await.unwrap();
        let loc = Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        };
        let entity = Entity::new(0, EntityType::Player, loc);
        entity_repo::insert(db.pool(), &entity).await.unwrap()
    }

    fn attack_ability() -> Ability {
        Ability {
            id: "attack".to_string(),
            name: "Attack".to_string(),
            description: Some("A basic attack.".to_string()),
            effects: vec![Effect {
                name: "damage".to_string(),
                effect_type: EffectType::AttributeUpdate {
                    attribute_id: "hp".to_string(),
                    value: -10,
                },
                trigger_info: TriggerInfo::Once,
                description: EffectDescription::default(),
                scope: EffectScope::default(),
            }],
            costs: vec![],
            modifiers: vec![],
            engagement_types: vec![EngagementType::Battle],
            role: AbilityRole::Attack,
            targets: vec![AbilityTargetType::Opponent],
        }
    }

    fn defend_ability() -> Ability {
        Ability {
            id: "defend".to_string(),
            name: "Defend".to_string(),
            description: None,
            effects: vec![],
            costs: vec![],
            modifiers: vec![],
            engagement_types: vec![EngagementType::Battle],
            role: AbilityRole::Defend,
            targets: vec![AbilityTargetType::SelfTarget],
        }
    }

    #[tokio::test]
    async fn upsert_and_find_by_entity() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        let ability = attack_ability();
        upsert(db.pool(), &ability).await.unwrap();
        set_entity_abilities(db.pool(), entity_id, &["attack"])
            .await
            .unwrap();

        let abilities = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert_eq!(abilities.len(), 1);
        assert_eq!(abilities[0].id, "attack");
        assert_eq!(abilities[0].effects.len(), 1);
        assert_eq!(abilities[0].engagement_types, vec![EngagementType::Battle]);
        assert_eq!(abilities[0].role, AbilityRole::Attack);
    }

    #[tokio::test]
    async fn upsert_preserves_defend_role() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        let ability = defend_ability();
        upsert(db.pool(), &ability).await.unwrap();
        set_entity_abilities(db.pool(), entity_id, &["defend"])
            .await
            .unwrap();

        let abilities = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert_eq!(abilities.len(), 1);
        assert_eq!(abilities[0].role, AbilityRole::Defend);
    }

    #[tokio::test]
    async fn upsert_is_idempotent() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        let ability = attack_ability();
        upsert(db.pool(), &ability).await.unwrap();
        upsert(db.pool(), &ability).await.unwrap();
        set_entity_abilities(db.pool(), entity_id, &["attack"])
            .await
            .unwrap();

        let abilities = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert_eq!(abilities.len(), 1);
    }

    #[tokio::test]
    async fn set_entity_abilities_replaces_existing() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        let ability = attack_ability();
        upsert(db.pool(), &ability).await.unwrap();

        set_entity_abilities(db.pool(), entity_id, &["attack"])
            .await
            .unwrap();
        set_entity_abilities(db.pool(), entity_id, &[])
            .await
            .unwrap();

        let abilities = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert!(abilities.is_empty());
    }

    #[tokio::test]
    async fn cascade_delete_on_entity_delete() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        let ability = attack_ability();
        upsert(db.pool(), &ability).await.unwrap();
        set_entity_abilities(db.pool(), entity_id, &["attack"])
            .await
            .unwrap();

        entity_repo::delete(db.pool(), entity_id).await.unwrap();

        let abilities = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert!(abilities.is_empty());
    }
}
