use sqlx::SqlitePool;

use crate::game::Location;
use crate::persistence::{dungeon_repo, room_repo};

use super::RoomApiError;

/// What a given script's `room.spawnEntity` call is allowed to target, matching the
/// script's own level: a room-feature script may only target its own room, a
/// dungeon script any room in that dungeon, a world script any room in that world.
#[derive(Debug, Clone)]
pub enum ScriptScope {
    Room(Location),
    Dungeon {
        world_id: String,
        dungeon_id: String,
    },
    World {
        world_id: String,
    },
}

impl ScriptScope {
    pub(super) async fn check(
        &self,
        pool: &SqlitePool,
        target: &Location,
    ) -> Result<(), RoomApiError> {
        match self {
            ScriptScope::Room(home) => (target == home)
                .then_some(())
                .ok_or_else(|| RoomApiError::out_of_scope(target)),
            ScriptScope::Dungeon { dungeon_id, .. } => {
                check_dungeon(pool, dungeon_id, target).await
            }
            ScriptScope::World { world_id } => check_world(pool, world_id, target).await,
        }
    }
}

async fn check_dungeon(
    pool: &SqlitePool,
    dungeon_id: &str,
    target: &Location,
) -> Result<(), RoomApiError> {
    if target.dungeon_id != dungeon_id {
        return Err(RoomApiError::out_of_scope(target));
    }
    room_exists(pool, dungeon_id, target).await
}

async fn check_world(
    pool: &SqlitePool,
    world_id: &str,
    target: &Location,
) -> Result<(), RoomApiError> {
    if target.world_id != world_id {
        return Err(RoomApiError::out_of_scope(target));
    }
    let dungeons = dungeon_repo::find_by_world(pool, world_id).await?;
    if !dungeons
        .iter()
        .any(|dungeon| dungeon.id == target.dungeon_id)
    {
        return Err(RoomApiError::out_of_scope(target));
    }
    room_exists(pool, &target.dungeon_id, target).await
}

async fn room_exists(
    pool: &SqlitePool,
    dungeon_id: &str,
    target: &Location,
) -> Result<(), RoomApiError> {
    room_repo::find_by_id(pool, dungeon_id, &target.room_id)
        .await?
        .ok_or_else(|| RoomApiError::out_of_scope(target))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::game::component::description::Description;
    use crate::game::{Dungeon, Room, World};
    use crate::persistence::{Database, dungeon_repo, room_repo, world_repo};

    use super::*;

    async fn seed(db: &Database) {
        world_repo::insert(db.pool(), &World::new("w1".to_string()))
            .await
            .unwrap();
        world_repo::insert(db.pool(), &World::new("w2".to_string()))
            .await
            .unwrap();
        dungeon_repo::insert(db.pool(), &Dungeon::new("d1".to_string()), "w1")
            .await
            .unwrap();
        dungeon_repo::insert(db.pool(), &Dungeon::new("d2".to_string()), "w2")
            .await
            .unwrap();
        room_repo::insert(
            db.pool(),
            &Room::new("r1".to_string(), Description::new(None)),
            "d1",
        )
        .await
        .unwrap();
    }

    fn room_in_d1() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    #[tokio::test]
    async fn room_scope_accepts_its_own_room() {
        let db = Database::connect_in_memory().await.unwrap();
        seed(&db).await;
        let scope = ScriptScope::Room(room_in_d1());

        assert!(scope.check(db.pool(), &room_in_d1()).await.is_ok());
    }

    #[tokio::test]
    async fn room_scope_rejects_a_different_room() {
        let db = Database::connect_in_memory().await.unwrap();
        seed(&db).await;
        let scope = ScriptScope::Room(room_in_d1());
        let other = Location {
            room_id: "r2".to_string(),
            ..room_in_d1()
        };

        assert!(scope.check(db.pool(), &other).await.is_err());
    }

    #[tokio::test]
    async fn dungeon_scope_accepts_a_room_in_that_dungeon() {
        let db = Database::connect_in_memory().await.unwrap();
        seed(&db).await;
        let scope = ScriptScope::Dungeon {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
        };

        assert!(scope.check(db.pool(), &room_in_d1()).await.is_ok());
    }

    #[tokio::test]
    async fn dungeon_scope_rejects_a_room_in_another_dungeon() {
        let db = Database::connect_in_memory().await.unwrap();
        seed(&db).await;
        let scope = ScriptScope::Dungeon {
            world_id: "w2".to_string(),
            dungeon_id: "d2".to_string(),
        };

        assert!(scope.check(db.pool(), &room_in_d1()).await.is_err());
    }

    #[tokio::test]
    async fn dungeon_scope_rejects_a_room_id_that_does_not_exist_in_that_dungeon() {
        let db = Database::connect_in_memory().await.unwrap();
        seed(&db).await;
        let scope = ScriptScope::Dungeon {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
        };
        let spoofed = Location {
            room_id: "does-not-exist".to_string(),
            ..room_in_d1()
        };

        assert!(scope.check(db.pool(), &spoofed).await.is_err());
    }

    #[tokio::test]
    async fn world_scope_accepts_a_room_in_a_dungeon_of_that_world() {
        let db = Database::connect_in_memory().await.unwrap();
        seed(&db).await;
        let scope = ScriptScope::World {
            world_id: "w1".to_string(),
        };

        assert!(scope.check(db.pool(), &room_in_d1()).await.is_ok());
    }

    #[tokio::test]
    async fn world_scope_rejects_a_room_in_another_world() {
        let db = Database::connect_in_memory().await.unwrap();
        seed(&db).await;
        let scope = ScriptScope::World {
            world_id: "w2".to_string(),
        };

        assert!(scope.check(db.pool(), &room_in_d1()).await.is_err());
    }

    #[tokio::test]
    async fn world_scope_rejects_a_room_id_that_does_not_exist_in_that_world() {
        let db = Database::connect_in_memory().await.unwrap();
        seed(&db).await;
        let scope = ScriptScope::World {
            world_id: "w1".to_string(),
        };
        let spoofed = Location {
            room_id: "does-not-exist".to_string(),
            ..room_in_d1()
        };

        assert!(scope.check(db.pool(), &spoofed).await.is_err());
    }
}
