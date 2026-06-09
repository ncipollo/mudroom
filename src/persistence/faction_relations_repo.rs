use sqlx::SqlitePool;

use crate::game::component::faction_relations::{FactionRelation, FactionRelations};
use crate::persistence::error::PersistenceError;

pub async fn set_entity_relations(
    pool: &SqlitePool,
    entity_id: i64,
    relations: &FactionRelations,
) -> Result<(), PersistenceError> {
    sqlx::query("DELETE FROM entity_faction_relations WHERE entity_id = ?")
        .bind(entity_id)
        .execute(pool)
        .await?;

    for (faction_id, relation) in &relations.factions {
        sqlx::query(
            "INSERT INTO entity_faction_relations (entity_id, faction_id, relation) VALUES (?, ?, ?)",
        )
        .bind(entity_id)
        .bind(faction_id)
        .bind(relation_to_str(relation))
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn find_by_entity(
    pool: &SqlitePool,
    entity_id: i64,
) -> Result<FactionRelations, PersistenceError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT faction_id, relation FROM entity_faction_relations WHERE entity_id = ?",
    )
    .bind(entity_id)
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
        let entity = Entity::new(0, EntityType::Character, loc);
        entity_repo::insert(db.pool(), &entity).await.unwrap()
    }

    #[tokio::test]
    async fn set_and_find_by_entity_round_trip() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;

        let mut relations = FactionRelations::default_for_monster();
        relations
            .factions
            .insert("bandits".to_string(), FactionRelation::Unfriendly);

        set_entity_relations(db.pool(), entity_id, &relations)
            .await
            .unwrap();

        let found = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert_eq!(found.player_relation(), &FactionRelation::Hostile);
        assert_eq!(found.monster_relation(), &FactionRelation::NonInteractive);
        assert_eq!(found.factions["bandits"], FactionRelation::Unfriendly);
    }

    #[tokio::test]
    async fn set_entity_relations_replaces_existing() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;

        set_entity_relations(
            db.pool(),
            entity_id,
            &FactionRelations::default_for_monster(),
        )
        .await
        .unwrap();

        set_entity_relations(
            db.pool(),
            entity_id,
            &FactionRelations::default_for_player(),
        )
        .await
        .unwrap();

        let found = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert_eq!(found.player_relation(), &FactionRelation::Friendly);
        assert_eq!(found.monster_relation(), &FactionRelation::Hostile);
    }

    #[tokio::test]
    async fn find_by_entity_returns_default_when_no_rows() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;

        let found = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert_eq!(found.player_relation(), &FactionRelation::NonInteractive);
        assert_eq!(found.monster_relation(), &FactionRelation::NonInteractive);
        assert!(found.factions.is_empty());
    }

    #[tokio::test]
    async fn cascade_delete_on_entity_delete() {
        let db = Database::connect_in_memory().await.unwrap();
        let entity_id = setup(&db).await;

        set_entity_relations(
            db.pool(),
            entity_id,
            &FactionRelations::default_for_monster(),
        )
        .await
        .unwrap();

        entity_repo::delete(db.pool(), entity_id).await.unwrap();

        let found = find_by_entity(db.pool(), entity_id).await.unwrap();
        assert_eq!(found.player_relation(), &FactionRelation::NonInteractive);
        assert!(found.factions.is_empty());
    }
}
