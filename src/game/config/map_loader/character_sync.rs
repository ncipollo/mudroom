use sqlx::SqlitePool;
use std::collections::HashMap;
use std::error::Error;

use crate::game::component::{Ability, Attribute, FactionRelations};
use crate::game::config::character_config::CharacterTypeConfig;
use crate::game::{CharacterConfig, CharacterType, Location, Room, Universe};
use crate::persistence::{
    ability_repo, character_effect_repo, character_repo, faction_relations_repo, faction_repo,
};

pub async fn load_characters_into_db(
    pool: &SqlitePool,
    universe: &Universe,
    character_configs: &HashMap<String, CharacterConfig>,
    ability_cache: &HashMap<String, Ability>,
) -> Result<(), Box<dyn Error>> {
    for world in universe.worlds.values() {
        for dungeon in world.dungeons.values() {
            for room in dungeon.rooms.values() {
                sync_room_characters(
                    pool,
                    &world.id,
                    &dungeon.id,
                    room,
                    character_configs,
                    ability_cache,
                )
                .await?;
            }
        }
    }
    Ok(())
}

async fn sync_room_characters(
    pool: &SqlitePool,
    world_id: &str,
    dungeon_id: &str,
    room: &Room,
    character_configs: &HashMap<String, CharacterConfig>,
    ability_cache: &HashMap<String, Ability>,
) -> Result<(), Box<dyn Error>> {
    for config_id in &room.entities {
        if let Some(config) = character_configs.get(config_id) {
            let name = config.name.as_deref().unwrap_or("unknown");
            let location = Location {
                world_id: world_id.to_string(),
                dungeon_id: dungeon_id.to_string(),
                room_id: room.id.clone(),
            };
            sync_character(pool, &location, config_id, name, config, ability_cache).await?;
        }
    }
    Ok(())
}

async fn sync_character(
    pool: &SqlitePool,
    location: &Location,
    config_id: &str,
    name: &str,
    config: &CharacterConfig,
    ability_cache: &HashMap<String, Ability>,
) -> Result<(), Box<dyn Error>> {
    let character_type = match config.entity_type {
        CharacterTypeConfig::Character => CharacterType::Character,
        CharacterTypeConfig::Enemy => CharacterType::Enemy,
    };
    let (character_id, is_new) = character_repo::insert_config_character_if_missing(
        pool,
        &character_type,
        location,
        config_id,
        config.description.text.as_deref(),
        name,
    )
    .await?;
    character_repo::update_battle_ai_type(pool, character_id, &config.battle_ai.ai_type).await?;
    if is_new {
        for effect in &config.entity_effects {
            character_effect_repo::insert(pool, character_id, effect).await?;
        }
    }
    if !config.attributes.is_empty() {
        sync_character_attributes(pool, character_id, config).await?;
    }
    let factions = effective_factions(&character_type, &config.factions);
    faction_repo::set_character_factions(pool, character_id, &factions).await?;
    let relations = effective_faction_relations(&character_type, config.faction_relations.as_ref());
    faction_relations_repo::set_character_relations(pool, character_id, &relations).await?;
    sync_character_abilities(pool, character_id, config, ability_cache).await?;
    Ok(())
}

async fn sync_character_abilities(
    pool: &SqlitePool,
    character_id: i64,
    config: &CharacterConfig,
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
        ability_repo::set_character_abilities(pool, character_id, &ids).await?;
    }
    Ok(())
}

async fn sync_character_attributes(
    pool: &SqlitePool,
    character_id: i64,
    config: &CharacterConfig,
) -> Result<(), Box<dyn Error>> {
    let existing = character_repo::find_by_id(pool, character_id).await?;
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
    character_repo::update_attributes(pool, character_id, &attrs).await?;
    Ok(())
}

fn effective_factions(character_type: &CharacterType, config_factions: &[String]) -> Vec<String> {
    if !config_factions.is_empty() {
        return config_factions.to_vec();
    }
    match character_type {
        CharacterType::Player => vec!["player".to_string()],
        CharacterType::Enemy => vec!["enemy".to_string()],
        _ => vec![],
    }
}

fn effective_faction_relations(
    character_type: &CharacterType,
    config_relations: Option<&FactionRelations>,
) -> FactionRelations {
    config_relations
        .cloned()
        .unwrap_or_else(|| match character_type {
            CharacterType::Player => FactionRelations::default_for_player(),
            CharacterType::Enemy => FactionRelations::default_for_enemy(),
            _ => FactionRelations::default(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::config::BattleAiConfig;
    use crate::game::config::character_config::{CharacterConfig, CharacterTypeConfig};
    use crate::game::{Description, Dungeon, Room, World};
    use crate::persistence::character_repo;
    use crate::persistence::database::Database;

    use super::super::universe_sync::load_map_into_db;

    fn make_universe_with_character() -> Universe {
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

    fn make_character_configs() -> HashMap<String, CharacterConfig> {
        let config = CharacterConfig {
            id: Some("entities/innkeeper".to_string()),
            name: Some("innkeeper".to_string()),
            entity_type: CharacterTypeConfig::Character,
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

    fn make_character_configs_with_attributes() -> HashMap<String, CharacterConfig> {
        use crate::game::config::character_config::StartingAttribute;
        let config = CharacterConfig {
            id: Some("entities/innkeeper".to_string()),
            name: Some("innkeeper".to_string()),
            entity_type: CharacterTypeConfig::Character,
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

    async fn load_innkeeper(db: &Database, configs: &HashMap<String, CharacterConfig>) {
        let universe = make_universe_with_character();
        load_map_into_db(db.pool(), &universe).await.unwrap();
        load_characters_into_db(db.pool(), &universe, configs, &HashMap::new())
            .await
            .unwrap();
    }

    async fn find_innkeeper_attrs(db: &Database) -> HashMap<String, Attribute> {
        let characters = character_repo::find_by_location(db.pool(), &innkeeper_location())
            .await
            .unwrap();
        characters.into_iter().next().unwrap().attributes
    }

    #[tokio::test]
    async fn load_characters_into_db_inserts_character() {
        let db = Database::connect_in_memory().await.unwrap();
        load_innkeeper(&db, &make_character_configs()).await;

        let characters = character_repo::find_by_location(db.pool(), &innkeeper_location())
            .await
            .unwrap();
        assert_eq!(characters.len(), 1);
        assert_eq!(
            characters[0].config_id.as_deref(),
            Some("entities/innkeeper")
        );
    }

    #[tokio::test]
    async fn load_characters_into_db_is_idempotent() {
        let db = Database::connect_in_memory().await.unwrap();
        let universe = make_universe_with_character();
        load_map_into_db(db.pool(), &universe).await.unwrap();
        let configs = make_character_configs();
        load_characters_into_db(db.pool(), &universe, &configs, &HashMap::new())
            .await
            .unwrap();
        load_characters_into_db(db.pool(), &universe, &configs, &HashMap::new())
            .await
            .unwrap();

        let characters = character_repo::find_by_location(db.pool(), &innkeeper_location())
            .await
            .unwrap();
        assert_eq!(characters.len(), 1);
    }

    #[tokio::test]
    async fn load_entities_populates_starting_attributes() {
        let db = Database::connect_in_memory().await.unwrap();
        load_innkeeper(&db, &make_character_configs_with_attributes()).await;

        let attrs = find_innkeeper_attrs(&db).await;
        assert_eq!(attrs["hp"], Attribute::new("hp".to_string(), 0, 100, 100));
        assert_eq!(attrs["mp"], Attribute::new("mp".to_string(), 0, 50, 50));
    }

    #[tokio::test]
    async fn load_entities_restores_empty_attributes_from_config() {
        let db = Database::connect_in_memory().await.unwrap();
        load_innkeeper(&db, &make_character_configs()).await;
        assert!(find_innkeeper_attrs(&db).await.is_empty());

        let universe = make_universe_with_character();
        load_characters_into_db(
            db.pool(),
            &universe,
            &make_character_configs_with_attributes(),
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
        use crate::game::config::character_config::StartingAttribute;

        let db = Database::connect_in_memory().await.unwrap();
        load_innkeeper(&db, &make_character_configs_with_attributes()).await;

        // Drain hp to 75 in DB
        let characters = character_repo::find_by_location(db.pool(), &innkeeper_location())
            .await
            .unwrap();
        let character_id = characters[0].id;
        let mut attrs = characters[0].attributes.clone();
        attrs.get_mut("hp").unwrap().current_value = 75;
        character_repo::update_attributes(db.pool(), character_id, &attrs)
            .await
            .unwrap();

        // Reload with tightened ranges: hp 10..90, mp 0..30
        let mut new_configs = HashMap::new();
        new_configs.insert(
            "entities/innkeeper".to_string(),
            CharacterConfig {
                id: Some("entities/innkeeper".to_string()),
                name: Some("innkeeper".to_string()),
                entity_type: CharacterTypeConfig::Character,
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
        let universe = make_universe_with_character();
        load_characters_into_db(db.pool(), &universe, &new_configs, &HashMap::new())
            .await
            .unwrap();

        let attrs = find_innkeeper_attrs(&db).await;
        // hp current_value 75 preserved, range updated to 10..90
        assert_eq!(attrs["hp"], Attribute::new("hp".to_string(), 10, 90, 75));
        // mp current_value 50 clamped to new max 30
        assert_eq!(attrs["mp"], Attribute::new("mp".to_string(), 0, 30, 30));
    }

    fn make_enemy_configs() -> HashMap<String, CharacterConfig> {
        let config = CharacterConfig {
            id: Some("entities/zombie".to_string()),
            name: Some("zombie".to_string()),
            entity_type: CharacterTypeConfig::Enemy,
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
        load_characters_into_db(db.pool(), &universe, &make_enemy_configs(), &HashMap::new())
            .await
            .unwrap();

        let characters = character_repo::find_by_location(db.pool(), &innkeeper_location())
            .await
            .unwrap();
        assert_eq!(characters.len(), 1);
        assert!(characters[0].factions.contains("enemy"));
    }

    #[tokio::test]
    async fn load_entities_writes_default_faction_relations_for_enemy() {
        use crate::game::component::faction_relations::FactionRelation;

        let db = Database::connect_in_memory().await.unwrap();
        setup_enemy_faction(&db).await;
        let universe = make_universe_with_enemy();
        load_map_into_db(db.pool(), &universe).await.unwrap();
        load_characters_into_db(db.pool(), &universe, &make_enemy_configs(), &HashMap::new())
            .await
            .unwrap();

        let characters = character_repo::find_by_location(db.pool(), &innkeeper_location())
            .await
            .unwrap();
        assert_eq!(characters.len(), 1);
        assert_eq!(
            characters[0].faction_relations.player_relation(),
            &FactionRelation::Hostile
        );
    }
}
