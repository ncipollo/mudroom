use sqlx::SqlitePool;

use crate::game::component::effect::{Effect, EffectDescription, EffectType, TriggerInfo};
use crate::persistence::error::PersistenceError;

type EffectRow = (i64, String, String, String, String);

pub async fn insert(
    pool: &SqlitePool,
    character_id: i64,
    effect: &Effect,
) -> Result<(), PersistenceError> {
    let effect_type_json =
        serde_json::to_string(&effect.effect_type).map_err(PersistenceError::Json)?;
    let trigger_info_json =
        serde_json::to_string(&effect.trigger_info).map_err(PersistenceError::Json)?;
    let description_json =
        serde_json::to_string(&effect.description).map_err(PersistenceError::Json)?;

    sqlx::query(
        "INSERT INTO character_effects (character_id, name, effect_type_json, trigger_info_json, description_json) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(character_id)
    .bind(&effect.name)
    .bind(effect_type_json)
    .bind(trigger_info_json)
    .bind(description_json)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_by_character(
    pool: &SqlitePool,
    character_id: i64,
) -> Result<Vec<Effect>, PersistenceError> {
    let rows: Vec<EffectRow> = sqlx::query_as(
        "SELECT id, name, effect_type_json, trigger_info_json, description_json FROM character_effects WHERE character_id = ?",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(
            |(_id, name, effect_type_json, trigger_info_json, description_json)| {
                let effect_type: EffectType =
                    serde_json::from_str(&effect_type_json).map_err(PersistenceError::Json)?;
                let trigger_info: TriggerInfo =
                    serde_json::from_str(&trigger_info_json).map_err(PersistenceError::Json)?;
                let description: EffectDescription =
                    serde_json::from_str(&description_json).map_err(PersistenceError::Json)?;
                Ok(Effect {
                    name,
                    effect_type,
                    trigger_info,
                    description,
                    scope: Default::default(),
                })
            },
        )
        .collect()
}

pub async fn delete_by_character(
    pool: &SqlitePool,
    character_id: i64,
) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM character_effects WHERE character_id = ?")
        .bind(character_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::effect::{EffectDescription, EffectType, TriggerInfo};
    use crate::game::{Character, CharacterType, Description, Dungeon, Location, Room, World};
    use crate::persistence::database::Database;
    use crate::persistence::{character_repo, dungeon_repo, room_repo, world_repo};

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
        let character = Character::new(0, CharacterType::Character, loc);
        character_repo::insert(db.pool(), &character).await.unwrap()
    }

    fn make_effect() -> Effect {
        Effect {
            name: "heal".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: 10,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription {
                start_description: Some("You feel healed.".to_string()),
                ..Default::default()
            },
            scope: Default::default(),
        }
    }

    #[tokio::test]
    async fn insert_and_find_by_character() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        let effect = make_effect();
        insert(db.pool(), character_id, &effect).await.unwrap();

        let found = find_by_character(db.pool(), character_id).await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0], effect);
    }

    #[tokio::test]
    async fn find_by_character_returns_empty_for_unknown() {
        let db = Database::connect_in_memory().await.unwrap();
        let found = find_by_character(db.pool(), 999).await.unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn delete_by_character_removes_effects() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        insert(db.pool(), character_id, &make_effect())
            .await
            .unwrap();
        delete_by_character(db.pool(), character_id).await.unwrap();

        let found = find_by_character(db.pool(), character_id).await.unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn cascade_delete_on_character_delete() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        insert(db.pool(), character_id, &make_effect())
            .await
            .unwrap();

        character_repo::delete(db.pool(), character_id)
            .await
            .unwrap();

        let found = find_by_character(db.pool(), character_id).await.unwrap();
        assert!(found.is_empty());
    }
}
