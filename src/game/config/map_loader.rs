use sqlx::SqlitePool;
use std::error::Error;

use crate::game::Universe;
use crate::persistence::{dungeon_repo, entity_repo, room_repo, server_state_repo, world_repo};

const LAST_MAP_LOAD_KEY: &str = "last_map_load_date";

pub async fn should_auto_load(pool: &SqlitePool) -> Result<bool, Box<dyn Error>> {
    let value = server_state_repo::get(pool, LAST_MAP_LOAD_KEY).await?;
    Ok(value.is_none())
}

pub async fn load_map_into_db(
    pool: &SqlitePool,
    universe: &Universe,
) -> Result<(), Box<dyn Error>> {
    upsert_universe(pool, universe).await?;
    cleanup_stale(pool, universe).await?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();
    server_state_repo::set(pool, LAST_MAP_LOAD_KEY, &timestamp).await?;
    Ok(())
}

async fn upsert_universe(pool: &SqlitePool, universe: &Universe) -> Result<(), Box<dyn Error>> {
    for world in universe.worlds.values() {
        world_repo::insert(pool, world).await?;
        for dungeon in world.dungeons.values() {
            dungeon_repo::insert(pool, dungeon, &world.id).await?;
            for room in dungeon.rooms.values() {
                room_repo::insert(pool, room, &dungeon.id).await?;
            }
        }
    }
    Ok(())
}

async fn cleanup_stale(pool: &SqlitePool, universe: &Universe) -> Result<(), Box<dyn Error>> {
    let db_worlds = world_repo::find_all(pool).await?;
    for db_world in db_worlds {
        if !universe.worlds.contains_key(&db_world.id) {
            delete_world_cascade(pool, &db_world.id).await?;
            world_repo::delete(pool, &db_world.id).await?;
            continue;
        }
        let universe_world = &universe.worlds[&db_world.id];
        let db_dungeons = dungeon_repo::find_by_world(pool, &db_world.id).await?;
        for db_dungeon in db_dungeons {
            if !universe_world.dungeons.contains_key(&db_dungeon.id) {
                delete_dungeon_cascade(pool, &db_dungeon.id).await?;
                dungeon_repo::delete(pool, &db_dungeon.id).await?;
                continue;
            }
            let universe_dungeon = &universe_world.dungeons[&db_dungeon.id];
            let db_rooms = room_repo::find_by_dungeon(pool, &db_dungeon.id).await?;
            for db_room in db_rooms {
                if !universe_dungeon.rooms.contains_key(&db_room.id) {
                    entity_repo::delete_by_room(pool, &db_room.id).await?;
                    room_repo::delete(pool, &db_room.id).await?;
                }
            }
        }
    }
    Ok(())
}

async fn delete_dungeon_cascade(pool: &SqlitePool, dungeon_id: &str) -> Result<(), Box<dyn Error>> {
    let rooms = room_repo::find_by_dungeon(pool, dungeon_id).await?;
    for room in rooms {
        entity_repo::delete_by_room(pool, &room.id).await?;
        room_repo::delete(pool, &room.id).await?;
    }
    Ok(())
}

async fn delete_world_cascade(pool: &SqlitePool, world_id: &str) -> Result<(), Box<dyn Error>> {
    let dungeons = dungeon_repo::find_by_world(pool, world_id).await?;
    for dungeon in dungeons {
        delete_dungeon_cascade(pool, &dungeon.id).await?;
        dungeon_repo::delete(pool, &dungeon.id).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Description, Dungeon, Room, World};
    use crate::persistence::database::Database;

    fn make_universe() -> Universe {
        let mut universe = Universe::default();
        let mut world = World::new("w1".to_string());
        let mut dungeon = Dungeon::new("d1".to_string());
        let room = Room::new(
            "r1".to_string(),
            Description::new(Some("A room.".to_string())),
        );
        dungeon.rooms.insert("r1".to_string(), room);
        world.dungeons.insert("d1".to_string(), dungeon);
        universe.worlds.insert("w1".to_string(), world);
        universe
    }

    #[tokio::test]
    async fn should_auto_load_returns_true_when_no_key() {
        let db = Database::connect_in_memory().await.unwrap();
        assert!(should_auto_load(db.pool()).await.unwrap());
    }

    #[tokio::test]
    async fn should_auto_load_returns_false_after_load() {
        let db = Database::connect_in_memory().await.unwrap();
        let universe = make_universe();
        load_map_into_db(db.pool(), &universe).await.unwrap();
        assert!(!should_auto_load(db.pool()).await.unwrap());
    }

    #[tokio::test]
    async fn load_map_into_db_upserts_world_dungeon_room() {
        let db = Database::connect_in_memory().await.unwrap();
        let universe = make_universe();
        load_map_into_db(db.pool(), &universe).await.unwrap();

        let world = world_repo::find_by_id(db.pool(), "w1").await.unwrap();
        assert!(world.is_some());
        let dungeon = dungeon_repo::find_by_id(db.pool(), "d1").await.unwrap();
        assert!(dungeon.is_some());
        let room = room_repo::find_by_id(db.pool(), "r1").await.unwrap();
        assert!(room.is_some());
    }

    #[tokio::test]
    async fn load_map_into_db_removes_stale_rooms() {
        let db = Database::connect_in_memory().await.unwrap();
        // Load universe with r1 and r2
        let mut universe = make_universe();
        {
            let dungeon = universe
                .worlds
                .get_mut("w1")
                .unwrap()
                .dungeons
                .get_mut("d1")
                .unwrap();
            dungeon.rooms.insert(
                "r2".to_string(),
                Room::new("r2".to_string(), Description::new(None)),
            );
        }
        load_map_into_db(db.pool(), &universe).await.unwrap();

        // Now reload with only r1
        let universe2 = make_universe();
        load_map_into_db(db.pool(), &universe2).await.unwrap();

        let room = room_repo::find_by_id(db.pool(), "r2").await.unwrap();
        assert!(room.is_none());
        let room1 = room_repo::find_by_id(db.pool(), "r1").await.unwrap();
        assert!(room1.is_some());
    }

    #[tokio::test]
    async fn load_map_into_db_removes_stale_worlds() {
        let db = Database::connect_in_memory().await.unwrap();
        let mut universe = make_universe();
        let mut extra_world = World::new("w2".to_string());
        let mut extra_dungeon = Dungeon::new("d2".to_string());
        let extra_room = Room::new("r2".to_string(), Description::new(None));
        extra_dungeon.rooms.insert("r2".to_string(), extra_room);
        extra_world.dungeons.insert("d2".to_string(), extra_dungeon);
        universe.worlds.insert("w2".to_string(), extra_world);
        load_map_into_db(db.pool(), &universe).await.unwrap();

        let universe2 = make_universe();
        load_map_into_db(db.pool(), &universe2).await.unwrap();

        let world = world_repo::find_by_id(db.pool(), "w2").await.unwrap();
        assert!(world.is_none());
    }
}
