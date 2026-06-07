use sqlx::SqlitePool;

use crate::game::component::Faction;
use crate::persistence::error::PersistenceError;

pub async fn upsert(pool: &SqlitePool, faction: &Faction) -> Result<(), PersistenceError> {
    sqlx::query("INSERT OR REPLACE INTO factions (id, name, description) VALUES (?, ?, ?)")
        .bind(&faction.id)
        .bind(&faction.name)
        .bind(&faction.description)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Faction>, PersistenceError> {
    let rows: Vec<(String, String, String)> =
        sqlx::query_as("SELECT id, name, description FROM factions")
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, description)| Faction {
            id,
            name,
            description,
        })
        .collect())
}

pub async fn find_by_entity(
    pool: &SqlitePool,
    entity_id: i64,
) -> Result<Vec<Faction>, PersistenceError> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT f.id, f.name, f.description FROM factions f \
         JOIN entity_factions ef ON f.id = ef.faction_id \
         WHERE ef.entity_id = ?",
    )
    .bind(entity_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, description)| Faction {
            id,
            name,
            description,
        })
        .collect())
}

pub async fn set_entity_factions(
    pool: &SqlitePool,
    entity_id: i64,
    faction_ids: &[String],
) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM entity_factions WHERE entity_id = ?")
        .bind(entity_id)
        .execute(pool)
        .await?;
    for faction_id in faction_ids {
        sqlx::query("INSERT INTO entity_factions (entity_id, faction_id) VALUES (?, ?)")
            .bind(entity_id)
            .bind(faction_id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn player_faction() -> Faction {
        Faction {
            id: "player".to_string(),
            name: "Player".to_string(),
            description: "All player characters.".to_string(),
        }
    }

    #[tokio::test]
    async fn upsert_and_find_all() {
        let db = Database::connect_in_memory().await.unwrap();
        upsert(db.pool(), &player_faction()).await.unwrap();
        let factions = find_all(db.pool()).await.unwrap();
        assert_eq!(factions.len(), 1);
        assert_eq!(factions[0].id, "player");
    }

    #[tokio::test]
    async fn upsert_is_idempotent() {
        let db = Database::connect_in_memory().await.unwrap();
        upsert(db.pool(), &player_faction()).await.unwrap();
        upsert(db.pool(), &player_faction()).await.unwrap();
        let factions = find_all(db.pool()).await.unwrap();
        assert_eq!(factions.len(), 1);
    }

    #[tokio::test]
    async fn set_and_find_by_entity() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        upsert(db.pool(), &player_faction()).await.unwrap();
        set_entity_factions(db.pool(), entity_id, &["player".to_string()])
            .await
            .unwrap();

        let factions = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert_eq!(factions.len(), 1);
        assert_eq!(factions[0].id, "player");
    }

    #[tokio::test]
    async fn set_entity_factions_replaces_existing() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        upsert(db.pool(), &player_faction()).await.unwrap();
        upsert(
            db.pool(),
            &Faction {
                id: "monster".to_string(),
                name: "Monster".to_string(),
                description: "Hostile creatures.".to_string(),
            },
        )
        .await
        .unwrap();

        set_entity_factions(db.pool(), entity_id, &["player".to_string()])
            .await
            .unwrap();
        set_entity_factions(db.pool(), entity_id, &["monster".to_string()])
            .await
            .unwrap();

        let factions = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert_eq!(factions.len(), 1);
        assert_eq!(factions[0].id, "monster");
    }

    #[tokio::test]
    async fn cascade_delete_on_entity_delete() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;
        upsert(db.pool(), &player_faction()).await.unwrap();
        set_entity_factions(db.pool(), entity_id, &["player".to_string()])
            .await
            .unwrap();

        entity_repo::delete(db.pool(), entity_id).await.unwrap();

        let factions = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert!(factions.is_empty());
    }
}
