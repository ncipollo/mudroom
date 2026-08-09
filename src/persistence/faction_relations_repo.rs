use sqlx::SqlitePool;

use crate::game::component::faction_relations::{FactionRelation, FactionRelations};
use crate::persistence::error::PersistenceError;

pub async fn set_character_relations(
    pool: &SqlitePool,
    character_id: i64,
    relations: &FactionRelations,
) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM character_faction_relations WHERE character_id = ?")
        .bind(character_id)
        .execute(pool)
        .await?;

    for (faction_id, relation) in &relations.factions {
        sqlx::query(
            "INSERT INTO character_faction_relations (character_id, faction_id, relation) VALUES (?, ?, ?)",
        )
        .bind(character_id)
        .bind(faction_id)
        .bind(relation_to_str(relation))
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn find_by_character(
    pool: &SqlitePool,
    character_id: i64,
) -> Result<FactionRelations, PersistenceError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT faction_id, relation FROM character_faction_relations WHERE character_id = ?",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?;

    let factions = rows
        .into_iter()
        .map(|(faction_id, relation_str)| (faction_id, parse_relation(&relation_str)))
        .collect();

    Ok(FactionRelations { factions })
}

fn relation_to_str(relation: &FactionRelation) -> &'static str {
    match relation {
        FactionRelation::Hostile => "hostile",
        FactionRelation::Unfriendly => "unfriendly",
        FactionRelation::Friendly => "friendly",
        FactionRelation::NonInteractive => "non_interactive",
    }
}

fn parse_relation(s: &str) -> FactionRelation {
    match s {
        "hostile" => FactionRelation::Hostile,
        "unfriendly" => FactionRelation::Unfriendly,
        "friendly" => FactionRelation::Friendly,
        _ => FactionRelation::NonInteractive,
    }
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
        let character = Character::new(0, CharacterType::Character, loc);
        character_repo::insert(db.pool(), &character).await.unwrap()
    }

    #[tokio::test]
    async fn set_and_find_by_character_round_trip() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        let mut relations = FactionRelations::default_for_enemy();
        relations
            .factions
            .insert("bandits".to_string(), FactionRelation::Unfriendly);

        set_character_relations(db.pool(), character_id, &relations)
            .await
            .unwrap();

        let found = find_by_character(db.pool(), character_id).await.unwrap();
        assert_eq!(found.player_relation(), &FactionRelation::Hostile);
        assert_eq!(found.enemy_relation(), &FactionRelation::NonInteractive);
        assert_eq!(found.factions["bandits"], FactionRelation::Unfriendly);
    }

    #[tokio::test]
    async fn set_character_relations_replaces_existing() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        set_character_relations(
            db.pool(),
            character_id,
            &FactionRelations::default_for_enemy(),
        )
        .await
        .unwrap();

        set_character_relations(
            db.pool(),
            character_id,
            &FactionRelations::default_for_player(),
        )
        .await
        .unwrap();

        let found = find_by_character(db.pool(), character_id).await.unwrap();
        assert_eq!(found.player_relation(), &FactionRelation::Friendly);
        assert_eq!(found.enemy_relation(), &FactionRelation::Hostile);
    }

    #[tokio::test]
    async fn find_by_character_returns_default_when_no_rows() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        let found = find_by_character(db.pool(), character_id).await.unwrap();
        assert_eq!(found.player_relation(), &FactionRelation::NonInteractive);
        assert_eq!(found.enemy_relation(), &FactionRelation::NonInteractive);
        assert!(found.factions.is_empty());
    }

    #[tokio::test]
    async fn cascade_delete_on_character_delete() {
        let db = Database::connect_in_memory().await.unwrap();
        let character_id = setup(&db).await;

        set_character_relations(
            db.pool(),
            character_id,
            &FactionRelations::default_for_enemy(),
        )
        .await
        .unwrap();

        character_repo::delete(db.pool(), character_id)
            .await
            .unwrap();

        let found = find_by_character(db.pool(), character_id).await.unwrap();
        assert_eq!(found.player_relation(), &FactionRelation::NonInteractive);
        assert!(found.factions.is_empty());
    }
}
