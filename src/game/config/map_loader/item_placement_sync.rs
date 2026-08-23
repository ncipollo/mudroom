use sqlx::SqlitePool;
use std::error::Error;

use crate::game::{Location, Room, Universe};
use crate::persistence::world_loot_repo;

pub async fn load_item_placements_into_db(
    pool: &SqlitePool,
    universe: &Universe,
) -> Result<(), Box<dyn Error>> {
    for world in universe.worlds.values() {
        for dungeon in world.dungeons.values() {
            for room in dungeon.rooms.values() {
                sync_room_items(pool, &world.id, &dungeon.id, room).await?;
            }
        }
    }
    Ok(())
}

async fn sync_room_items(
    pool: &SqlitePool,
    world_id: &str,
    dungeon_id: &str,
    room: &Room,
) -> Result<(), Box<dyn Error>> {
    for item_definition_id in &room.items {
        let location = Location {
            world_id: world_id.to_string(),
            dungeon_id: dungeon_id.to_string(),
            room_id: room.id.clone(),
        };
        world_loot_repo::insert_config_loot_if_missing(pool, &location, item_definition_id).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::{EquippedBonuses, ItemDefinition, ItemUseType};
    use crate::game::{Description, Dungeon, World};
    use crate::persistence::database::Database;
    use crate::persistence::{item_repo, world_loot_repo};

    use super::super::universe_sync::load_map_into_db;

    fn make_universe_with_item() -> Universe {
        let mut universe = Universe::default();
        let mut world = World::new("w1".to_string());
        let mut dungeon = Dungeon::new("d1".to_string());
        let mut room = Room::new(
            "r1".to_string(),
            Description::new(Some("A room.".to_string())),
        );
        room.items.push("health_tonic".to_string());
        dungeon.rooms.insert("r1".to_string(), room);
        world.dungeons.insert("d1".to_string(), dungeon);
        universe.worlds.insert("w1".to_string(), world);
        universe
    }

    fn item_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    async fn setup_item_definition(db: &Database) {
        let def = ItemDefinition {
            id: "health_tonic".to_string(),
            name: "Health Tonic".to_string(),
            description: Description::new(Some("A restorative brew.".to_string())),
            use_type: ItemUseType::Used,
            item_type: "medicine".to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects: vec![],
        };
        item_repo::upsert_definition(db.pool(), &def).await.unwrap();
    }

    #[tokio::test]
    async fn load_item_placements_into_db_seeds_world_loot() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_item_definition(&db).await;
        let universe = make_universe_with_item();
        load_map_into_db(db.pool(), &universe).await.unwrap();

        load_item_placements_into_db(db.pool(), &universe)
            .await
            .unwrap();

        let loot = world_loot_repo::find_by_location(db.pool(), &item_location())
            .await
            .unwrap();
        assert_eq!(loot.len(), 1);
        assert_eq!(loot[0].item_definition_id, "health_tonic");
    }

    #[tokio::test]
    async fn load_item_placements_into_db_is_idempotent() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_item_definition(&db).await;
        let universe = make_universe_with_item();
        load_map_into_db(db.pool(), &universe).await.unwrap();

        load_item_placements_into_db(db.pool(), &universe)
            .await
            .unwrap();
        load_item_placements_into_db(db.pool(), &universe)
            .await
            .unwrap();

        let loot = world_loot_repo::find_by_location(db.pool(), &item_location())
            .await
            .unwrap();
        assert_eq!(loot.len(), 1);
    }
}
