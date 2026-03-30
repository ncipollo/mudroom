use sqlx::SqlitePool;
use std::collections::HashMap;
use std::error::Error;

use crate::game::component::Attribute;
use crate::game::config::entity_config::EntityTypeConfig;
use crate::game::{EntityConfig, EntityType, Location, Universe};
use crate::persistence::{
    dungeon_repo, entity_effect_repo, entity_repo, room_repo, server_state_repo, world_repo,
};

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

pub async fn load_entities_into_db(
    pool: &SqlitePool,
    universe: &Universe,
    entity_configs: &HashMap<String, EntityConfig>,
) -> Result<(), Box<dyn Error>> {
    for world in universe.worlds.values() {
        for dungeon in world.dungeons.values() {
            for room in dungeon.rooms.values() {
                for config_id in &room.entities {
                    let Some(config) = entity_configs.get(config_id) else {
                        continue;
                    };
                    let entity_type = match config.entity_type {
                        EntityTypeConfig::Character => EntityType::Character,
                        EntityTypeConfig::Object => EntityType::Object,
                    };
                    let location = Location {
                        world_id: world.id.clone(),
                        dungeon_id: dungeon.id.clone(),
                        room_id: room.id.clone(),
                    };

                    let (entity_id, is_new) = entity_repo::insert_config_entity_if_missing(
                        pool,
                        &entity_type,
                        &location,
                        config_id,
                    )
                    .await?;

                    if is_new {
                        for effect in &config.entity_effects {
                            entity_effect_repo::insert(pool, entity_id, effect).await?;
                        }
                    }

                    if !config.attributes.is_empty() {
                        sync_entity_attributes(pool, entity_id, config).await?;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn sync_entity_attributes(
    pool: &SqlitePool,
    entity_id: i64,
    config: &EntityConfig,
) -> Result<(), Box<dyn Error>> {
    let existing = entity_repo::find_by_id(pool, entity_id).await?;
    let db_attrs = existing.map(|e| e.attributes).unwrap_or_default();
    let attrs: HashMap<String, Attribute> = config
        .attributes
        .iter()
        .map(|sa| {
            let current_value = db_attrs
                .get(&sa.definition_id)
                .map(|a| a.current_value.clamp(sa.min_value, sa.max_value))
                .unwrap_or(sa.current_value);
            (
                sa.definition_id.clone(),
                Attribute::new(
                    sa.definition_id.clone(),
                    sa.min_value,
                    sa.max_value,
                    current_value,
                ),
            )
        })
        .collect();
    entity_repo::update_attributes(pool, entity_id, &attrs).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Description, Dungeon, EntityConfig, Room, World};
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
        let room = room_repo::find_by_id(db.pool(), "d1", "r1").await.unwrap();
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

        let room = room_repo::find_by_id(db.pool(), "d1", "r2").await.unwrap();
        assert!(room.is_none());
        let room1 = room_repo::find_by_id(db.pool(), "d1", "r1").await.unwrap();
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

    fn make_universe_with_entity() -> Universe {
        let mut universe = Universe::default();
        let mut world = World::new("w1".to_string());
        let mut dungeon = Dungeon::new("d1".to_string());
        let mut room = Room::new(
            "r1".to_string(),
            Description::new(Some("A room.".to_string())),
        );
        room.entities.push("entities/innkeeper".to_string());
        dungeon.rooms.insert("r1".to_string(), room);
        world.dungeons.insert("d1".to_string(), dungeon);
        universe.worlds.insert("w1".to_string(), world);
        universe
    }

    fn make_entity_configs() -> HashMap<String, EntityConfig> {
        use crate::game::config::entity_config::{EntityConfig, EntityTypeConfig};
        let config = EntityConfig {
            id: Some("entities/innkeeper".to_string()),
            entity_type: EntityTypeConfig::Character,
            description: None,
            persona: None,
            attributes: vec![],
            entity_effects: vec![],
        };
        let mut map = HashMap::new();
        map.insert("entities/innkeeper".to_string(), config);
        map
    }

    fn make_entity_configs_with_attributes() -> HashMap<String, EntityConfig> {
        use crate::game::config::entity_config::{
            EntityConfig, EntityTypeConfig, StartingAttribute,
        };
        let config = EntityConfig {
            id: Some("entities/innkeeper".to_string()),
            entity_type: EntityTypeConfig::Character,
            description: None,
            persona: None,
            attributes: vec![
                StartingAttribute {
                    definition_id: "hp".to_string(),
                    min_value: 0,
                    max_value: 100,
                    current_value: 100,
                },
                StartingAttribute {
                    definition_id: "mp".to_string(),
                    min_value: 0,
                    max_value: 50,
                    current_value: 50,
                },
            ],
            entity_effects: vec![],
        };
        let mut map = HashMap::new();
        map.insert("entities/innkeeper".to_string(), config);
        map
    }

    fn innkeeper_location() -> crate::game::Location {
        crate::game::Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    async fn load_innkeeper(db: &Database, configs: &HashMap<String, EntityConfig>) {
        let universe = make_universe_with_entity();
        load_map_into_db(db.pool(), &universe).await.unwrap();
        load_entities_into_db(db.pool(), &universe, configs)
            .await
            .unwrap();
    }

    async fn find_innkeeper_attrs(db: &Database) -> HashMap<String, Attribute> {
        let entities = entity_repo::find_by_location(db.pool(), &innkeeper_location())
            .await
            .unwrap();
        entities.into_iter().next().unwrap().attributes
    }

    #[tokio::test]
    async fn load_entities_into_db_inserts_entity() {
        let db = Database::connect_in_memory().await.unwrap();
        load_innkeeper(&db, &make_entity_configs()).await;

        let entities = entity_repo::find_by_location(db.pool(), &innkeeper_location())
            .await
            .unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].config_id.as_deref(), Some("entities/innkeeper"));
    }

    #[tokio::test]
    async fn load_entities_into_db_is_idempotent() {
        let db = Database::connect_in_memory().await.unwrap();
        let universe = make_universe_with_entity();
        load_map_into_db(db.pool(), &universe).await.unwrap();
        let configs = make_entity_configs();
        load_entities_into_db(db.pool(), &universe, &configs)
            .await
            .unwrap();
        load_entities_into_db(db.pool(), &universe, &configs)
            .await
            .unwrap();

        let entities = entity_repo::find_by_location(db.pool(), &innkeeper_location())
            .await
            .unwrap();
        assert_eq!(entities.len(), 1);
    }

    #[tokio::test]
    async fn load_entities_populates_starting_attributes() {
        let db = Database::connect_in_memory().await.unwrap();
        load_innkeeper(&db, &make_entity_configs_with_attributes()).await;

        let attrs = find_innkeeper_attrs(&db).await;
        assert_eq!(attrs["hp"], Attribute::new("hp".to_string(), 0, 100, 100));
        assert_eq!(attrs["mp"], Attribute::new("mp".to_string(), 0, 50, 50));
    }

    #[tokio::test]
    async fn load_entities_restores_empty_attributes_from_config() {
        let db = Database::connect_in_memory().await.unwrap();
        load_innkeeper(&db, &make_entity_configs()).await;
        assert!(find_innkeeper_attrs(&db).await.is_empty());

        let universe = make_universe_with_entity();
        load_entities_into_db(db.pool(), &universe, &make_entity_configs_with_attributes())
            .await
            .unwrap();

        let attrs = find_innkeeper_attrs(&db).await;
        assert_eq!(attrs["hp"], Attribute::new("hp".to_string(), 0, 100, 100));
        assert_eq!(attrs["mp"], Attribute::new("mp".to_string(), 0, 50, 50));
    }

    #[tokio::test]
    async fn load_entities_preserves_current_value_and_updates_min_max() {
        use crate::game::config::entity_config::{
            EntityConfig, EntityTypeConfig, StartingAttribute,
        };

        let db = Database::connect_in_memory().await.unwrap();
        load_innkeeper(&db, &make_entity_configs_with_attributes()).await;

        // Drain hp to 75 in DB
        let entities = entity_repo::find_by_location(db.pool(), &innkeeper_location())
            .await
            .unwrap();
        let entity_id = entities[0].id;
        let mut attrs = entities[0].attributes.clone();
        attrs.get_mut("hp").unwrap().current_value = 75;
        entity_repo::update_attributes(db.pool(), entity_id, &attrs)
            .await
            .unwrap();

        // Reload with tightened ranges: hp 10..90, mp 0..30
        let mut new_configs = HashMap::new();
        new_configs.insert(
            "entities/innkeeper".to_string(),
            EntityConfig {
                id: Some("entities/innkeeper".to_string()),
                entity_type: EntityTypeConfig::Character,
                description: None,
                persona: None,
                attributes: vec![
                    StartingAttribute {
                        definition_id: "hp".to_string(),
                        min_value: 10,
                        max_value: 90,
                        current_value: 90,
                    },
                    StartingAttribute {
                        definition_id: "mp".to_string(),
                        min_value: 0,
                        max_value: 30,
                        current_value: 30,
                    },
                ],
                entity_effects: vec![],
            },
        );
        let universe = make_universe_with_entity();
        load_entities_into_db(db.pool(), &universe, &new_configs)
            .await
            .unwrap();

        let attrs = find_innkeeper_attrs(&db).await;
        // hp current_value 75 preserved, range updated to 10..90
        assert_eq!(attrs["hp"], Attribute::new("hp".to_string(), 10, 90, 75));
        // mp current_value 50 clamped to new max 30
        assert_eq!(attrs["mp"], Attribute::new("mp".to_string(), 0, 30, 30));
    }
}
