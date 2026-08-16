use crate::game::component::description::Description;
use crate::game::{Dungeon, Location, Room, World};
use crate::persistence::database::Database;
use crate::persistence::{dungeon_repo, room_repo, world_repo};

pub(super) async fn setup(db: &Database) {
    let world = World::new("w1".to_string());
    world_repo::insert(db.pool(), &world).await.unwrap();
    let dungeon = Dungeon::new("d1".to_string());
    dungeon_repo::insert(db.pool(), &dungeon, "w1")
        .await
        .unwrap();
    let room = Room::new("r1".to_string(), Description::new(None));
    room_repo::insert(db.pool(), &room, "d1").await.unwrap();
}

pub(super) fn test_location() -> Location {
    Location {
        world_id: "w1".to_string(),
        dungeon_id: "d1".to_string(),
        room_id: "r1".to_string(),
    }
}
