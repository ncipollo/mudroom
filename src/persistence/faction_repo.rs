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

pub async fn find_by_character(
    pool: &SqlitePool,
    character_id: i64,
) -> Result<Vec<Faction>, PersistenceError> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT f.id, f.name, f.description FROM factions f \
         JOIN character_factions ef ON f.id = ef.faction_id \
         WHERE ef.character_id = ?",
    )
    .bind(character_id)
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

pub async fn set_character_factions(
    pool: &SqlitePool,
    character_id: i64,
    faction_ids: &[String],
) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM character_factions WHERE character_id = ?")
        .bind(character_id)
        .execute(pool)
        .await?;
    for faction_id in faction_ids {
        sqlx::query("INSERT INTO character_factions (character_id, faction_id) VALUES (?, ?)")
            .bind(character_id)
            .bind(faction_id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let character = Character::new(0, CharacterType::Player, loc);
        character_repo::insert(db.pool(), &character).await.unwrap()
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
    async fn set_and_find_by_character() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        upsert(db.pool(), &player_faction()).await.unwrap();
        set_character_factions(db.pool(), character_id, &["player".to_string()])
            .await
            .unwrap();

        let factions = find_by_character(db.pool(), character_id).await.unwrap();
        assert_eq!(factions.len(), 1);
        assert_eq!(factions[0].id, "player");
    }

    #[tokio::test]
    async fn set_character_factions_replaces_existing() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
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

        set_character_factions(db.pool(), character_id, &["player".to_string()])
            .await
            .unwrap();
        set_character_factions(db.pool(), character_id, &["monster".to_string()])
            .await
            .unwrap();

        let factions = find_by_character(db.pool(), character_id).await.unwrap();
        assert_eq!(factions.len(), 1);
        assert_eq!(factions[0].id, "monster");
    }

    #[tokio::test]
    async fn cascade_delete_on_character_delete() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;
        upsert(db.pool(), &player_faction()).await.unwrap();
        set_character_factions(db.pool(), character_id, &["player".to_string()])
            .await
            .unwrap();

        character_repo::delete(db.pool(), character_id)
            .await
            .unwrap();

        let factions = find_by_character(db.pool(), character_id).await.unwrap();
        assert!(factions.is_empty());
    }
}
