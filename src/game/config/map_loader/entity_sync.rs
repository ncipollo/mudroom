use sqlx::SqlitePool;
use std::collections::HashMap;
use std::error::Error;

use crate::game::component::{Ability, Attribute, FactionRelations};
use crate::game::config::entity_config::EntityTypeConfig;
use crate::game::{EntityConfig, EntityType, Location, Room, Universe};
use crate::persistence::{
    ability_repo, entity_effect_repo, entity_repo, faction_relations_repo, faction_repo,
};

pub async fn load_entities_into_db(
    pool: &SqlitePool,
    universe: &Universe,
    entity_configs: &HashMap<String, EntityConfig>,
    ability_cache: &HashMap<String, Ability>,
) -> Result<(), Box<dyn Error>> {
    for world in universe.worlds.values() {
        for dungeon in world.dungeons.values() {
            for room in dungeon.rooms.values() {
                sync_room_entities(
                    pool,
                    &world.id,
                    &dungeon.id,
                    room,
                    entity_configs,
                    ability_cache,
                )
                .await?;
            }
        }
    }
    Ok(())
}

async fn sync_room_entities(
    pool: &SqlitePool,
    world_id: &str,
    dungeon_id: &str,
    room: &Room,
    entity_configs: &HashMap<String, EntityConfig>,
    ability_cache: &HashMap<String, Ability>,
) -> Result<(), Box<dyn Error>> {
    for config_id in &room.entities {
        if let Some(config) = entity_configs.get(config_id) {
            let name = config.name.as_deref().unwrap_or("unknown");
            let location = Location {
                world_id: world_id.to_string(),
                dungeon_id: dungeon_id.to_string(),
                room_id: room.id.clone(),
            };
            sync_entity(pool, &location, config_id, name, config, ability_cache).await?;
        }
    }
    Ok(())
}

async fn sync_entity(
    pool: &SqlitePool,
    location: &Location,
    config_id: &str,
    name: &str,
    config: &EntityConfig,
    ability_cache: &HashMap<String, Ability>,
) -> Result<(), Box<dyn Error>> {
    let entity_type = match config.entity_type {
        EntityTypeConfig::Character => EntityType::Character,
        EntityTypeConfig::Enemy => EntityType::Enemy,
        EntityTypeConfig::Object => EntityType::Object,
    };
    let (entity_id, is_new) = entity_repo::insert_config_entity_if_missing(
        pool,
        &entity_type,
        location,
        config_id,
        config.description.text.as_deref(),
        name,
    )
    .await?;
    entity_repo::update_battle_ai_type(pool, entity_id, &config.battle_ai.ai_type).await?;
    if is_new {
        for effect in &config.entity_effects {
            entity_effect_repo::insert(pool, entity_id, effect).await?;
        }
    }
    if !config.attributes.is_empty() {
        sync_entity_attributes(pool, entity_id, config).await?;
    }
    let factions = effective_factions(&entity_type, &config.factions);
    faction_repo::set_entity_factions(pool, entity_id, &factions).await?;
    let relations = effective_faction_relations(&entity_type, config.faction_relations.as_ref());
    faction_relations_repo::set_entity_relations(pool, entity_id, &relations).await?;
    sync_entity_abilities(pool, entity_id, config, ability_cache).await?;
    Ok(())
}

async fn sync_entity_abilities(
    pool: &SqlitePool,
    entity_id: i64,
    config: &EntityConfig,
    ability_cache: &HashMap<String, Ability>,
) -> Result<(), Box<dyn Error>> {
    let abilities: Vec<&Ability> = config
        .innate_abilities
        .iter()
        .filter_map(|r| {
            let ability = ability_cache.get(&r.0);
            if ability.is_none() {
                tracing::warn!("ability not found in cache: {}", r.0);
            }
            ability
        })
        .collect();
    for ability in &abilities {
        ability_repo::upsert(pool, ability).await?;
    }
    if !abilities.is_empty() {
        let ids: Vec<&str> = abilities.iter().map(|a| a.id.as_str()).collect();
        ability_repo::set_entity_abilities(pool, entity_id, &ids).await?;
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

fn effective_factions(entity_type: &EntityType, config_factions: &[String]) -> Vec<String> {
    if !config_factions.is_empty() {
        return config_factions.to_vec();
    }
    match entity_type {
        EntityType::Player => vec!["player".to_string()],
        EntityType::Enemy => vec!["enemy".to_string()],
        _ => vec![],
    }
}

fn effective_faction_relations(
    entity_type: &EntityType,
    config_relations: Option<&FactionRelations>,
) -> FactionRelations {
    config_relations
        .cloned()
        .unwrap_or_else(|| match entity_type {
            EntityType::Player => FactionRelations::default_for_player(),
            EntityType::Enemy => FactionRelations::default_for_enemy(),
            _ => FactionRelations::default(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::config::BattleAiConfig;
    use crate::game::config::entity_config::{EntityConfig, EntityTypeConfig};
    use crate::game::{Description, Dungeon, Room, World};
    use crate::persistence::database::Database;
    use crate::persistence::entity_repo;

    use super::super::universe_sync::load_map_into_db;

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
        let config = EntityConfig {
            id: Some("entities/innkeeper".to_string()),
            name: Some("innkeeper".to_string()),
            entity_type: EntityTypeConfig::Character,
            description: Description::default(),
            persona: None,
            attributes: vec![],
            entity_effects: vec![],
            innate_abilities: vec![],
            factions: vec![],
            faction_relations: None,
            battle_ai: BattleAiConfig::default(),
        };
        let mut map = HashMap::new();
        map.insert("entities/innkeeper".to_string(), config);
        map
    }

    fn make_entity_configs_with_attributes() -> HashMap<String, EntityConfig> {
        use crate::game::config::entity_config::StartingAttribute;
        let config = EntityConfig {
            id: Some("entities/innkeeper".to_string()),
            name: Some("innkeeper".to_string()),
            entity_type: EntityTypeConfig::Character,
            description: Description::default(),
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
            innate_abilities: vec![],
            factions: vec![],
            faction_relations: None,
            battle_ai: BattleAiConfig::default(),
        };
        let mut map = HashMap::new();
        map.insert("entities/innkeeper".to_string(), config);
        map
    }

    fn innkeeper_location() -> Location {
        Location {
            world_id: "w1".to_string(),
            dungeon_id: "d1".to_string(),
            room_id: "r1".to_string(),
        }
    }

    async fn load_innkeeper(db: &Database, configs: &HashMap<String, EntityConfig>) {
        let universe = make_universe_with_entity();
        load_map_into_db(db.pool(), &universe).await.unwrap();
        load_entities_into_db(db.pool(), &universe, configs, &HashMap::new())
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
        load_entities_into_db(db.pool(), &universe, &configs, &HashMap::new())
            .await
            .unwrap();
        load_entities_into_db(db.pool(), &universe, &configs, &HashMap::new())
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
        load_entities_into_db(
            db.pool(),
            &universe,
            &make_entity_configs_with_attributes(),
            &HashMap::new(),
        )
        .await
        .unwrap();

        let attrs = find_innkeeper_attrs(&db).await;
        assert_eq!(attrs["hp"], Attribute::new("hp".to_string(), 0, 100, 100));
        assert_eq!(attrs["mp"], Attribute::new("mp".to_string(), 0, 50, 50));
    }

    #[tokio::test]
    async fn load_entities_preserves_current_value_and_updates_min_max() {
        use crate::game::config::entity_config::StartingAttribute;

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
                name: Some("innkeeper".to_string()),
                entity_type: EntityTypeConfig::Character,
                description: Description::default(),
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
                innate_abilities: vec![],
                factions: vec![],
                faction_relations: None,
                battle_ai: BattleAiConfig::default(),
            },
        );
        let universe = make_universe_with_entity();
        load_entities_into_db(db.pool(), &universe, &new_configs, &HashMap::new())
            .await
            .unwrap();

        let attrs = find_innkeeper_attrs(&db).await;
        // hp current_value 75 preserved, range updated to 10..90
        assert_eq!(attrs["hp"], Attribute::new("hp".to_string(), 10, 90, 75));
        // mp current_value 50 clamped to new max 30
        assert_eq!(attrs["mp"], Attribute::new("mp".to_string(), 0, 30, 30));
    }

    fn make_enemy_configs() -> HashMap<String, EntityConfig> {
        let config = EntityConfig {
            id: Some("entities/zombie".to_string()),
            name: Some("zombie".to_string()),
            entity_type: EntityTypeConfig::Enemy,
            description: Description::default(),
            persona: None,
            attributes: vec![],
            entity_effects: vec![],
            innate_abilities: vec![],
            factions: vec![],
            faction_relations: None,
            battle_ai: BattleAiConfig::default(),
        };
        let mut map = HashMap::new();
        map.insert("entities/zombie".to_string(), config);
        map
    }

    fn make_universe_with_enemy() -> Universe {
        let mut universe = Universe::default();
        let mut world = World::new("w1".to_string());
        let mut dungeon = Dungeon::new("d1".to_string());
        let mut room = Room::new(
            "r1".to_string(),
            Description::new(Some("A room.".to_string())),
        );
        room.entities.push("entities/zombie".to_string());
        dungeon.rooms.insert("r1".to_string(), room);
        world.dungeons.insert("d1".to_string(), dungeon);
        universe.worlds.insert("w1".to_string(), world);
        universe
    }

    async fn setup_enemy_faction(db: &Database) {
        use crate::game::component::Faction;
        faction_repo::upsert(
            db.pool(),
            &Faction {
                id: "enemy".to_string(),
                name: "Enemy".to_string(),
                description: "Hostile creatures of the world.".to_string(),
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn load_entities_writes_default_factions_for_enemy() {
        let db = Database::connect_in_memory().await.unwrap();
        setup_enemy_faction(&db).await;
        let universe = make_universe_with_enemy();
        load_map_into_db(db.pool(), &universe).await.unwrap();
        load_entities_into_db(db.pool(), &universe, &make_enemy_configs(), &HashMap::new())
            .await
            .unwrap();

        let entities = entity_repo::find_by_location(db.pool(), &innkeeper_location())
            .await
            .unwrap();
        assert_eq!(entities.len(), 1);
        assert!(entities[0].factions.contains("enemy"));
    }

    #[tokio::test]
    async fn load_entities_writes_default_faction_relations_for_enemy() {
        use crate::game::component::faction_relations::FactionRelation;

        let db = Database::connect_in_memory().await.unwrap();
        setup_enemy_faction(&db).await;
        let universe = make_universe_with_enemy();
        load_map_into_db(db.pool(), &universe).await.unwrap();
        load_entities_into_db(db.pool(), &universe, &make_enemy_configs(), &HashMap::new())
            .await
            .unwrap();

        let entities = entity_repo::find_by_location(db.pool(), &innkeeper_location())
            .await
            .unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(
            entities[0].faction_relations.player_relation(),
            &FactionRelation::Hostile
        );
    }
}
