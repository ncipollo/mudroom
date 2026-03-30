use std::collections::{HashMap, HashSet};
use std::path::Path;

use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tokio::sync::broadcast;

use crate::game::config::{AttributeConfig, EntityConfig, MudConfig, load_entity_configs};
use crate::game::engagement::Engagements;
use crate::game::entity::Entity;
use crate::game::mailbox::Mailboxes;
use crate::game::messaging::PlayerMessage;
use crate::game::player::Player;
use crate::persistence::{PersistenceError, entity_repo};

pub struct GameState {
    pub attribute_config: AttributeConfig,
    pub mud_config: MudConfig,
    pub entity_configs: HashMap<String, EntityConfig>,
    pub active_entities: RwLock<HashMap<i64, Entity>>,
    pub active_dungeons: RwLock<HashSet<(String, String)>>,
    pub engagements: Engagements,
    pub mailboxes: Mailboxes,
    pub active_players: RwLock<HashMap<String, Player>>,
    pub message_tx: broadcast::Sender<PlayerMessage>,
}

impl GameState {
    pub fn load(config_dir: Option<&Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let attribute_config = if let Some(dir) = config_dir {
            let path = dir.join("attributes.toml");
            if path.exists() {
                AttributeConfig::load(&path)?
            } else {
                AttributeConfig::default_config()
            }
        } else {
            AttributeConfig::default_config()
        };

        let mud_config = if let Some(dir) = config_dir {
            let path = dir.join("mud.toml");
            if path.exists() {
                MudConfig::load(&path)?
            } else {
                MudConfig::default_config()
            }
        } else {
            MudConfig::default_config()
        };

        let entity_configs = if let Some(dir) = config_dir {
            load_entity_configs(dir).unwrap_or_default()
        } else {
            HashMap::new()
        };

        let (message_tx, _) = broadcast::channel::<PlayerMessage>(512);

        Ok(Self {
            attribute_config,
            mud_config,
            entity_configs,
            active_entities: RwLock::new(HashMap::new()),
            active_dungeons: RwLock::new(HashSet::new()),
            engagements: Engagements::new(),
            mailboxes: Mailboxes::new(),
            active_players: RwLock::new(HashMap::new()),
            message_tx,
        })
    }

    /// Recomputes active dungeons from current player locations, then syncs active_entities to
    /// include all config entities in those dungeons.
    pub async fn sync_active_entities(&self, pool: &SqlitePool) -> Result<(), PersistenceError> {
        // Compute the dungeons where players are currently located.
        let new_active_dungeons: HashSet<(String, String)> = {
            let entities = self.active_entities.read().await;
            let players = self.active_players.read().await;
            players
                .values()
                .filter_map(|p| entities.get(&p.entity_id))
                .map(|e| (e.location.world_id.clone(), e.location.dungeon_id.clone()))
                .collect()
        };

        let old_active_dungeons = self.active_dungeons.read().await.clone();

        if new_active_dungeons == old_active_dungeons {
            return Ok(());
        }

        let added: Vec<(String, String)> = new_active_dungeons
            .difference(&old_active_dungeons)
            .cloned()
            .collect();
        let removed: HashSet<(String, String)> = old_active_dungeons
            .difference(&new_active_dungeons)
            .cloned()
            .collect();

        // Fetch new entities from DB before acquiring write lock.
        let mut incoming: Vec<Entity> = Vec::new();
        for (world_id, dungeon_id) in &added {
            let mut dungeon_entities =
                entity_repo::find_config_entities_by_dungeon(pool, world_id, dungeon_id).await?;
            for entity in &mut dungeon_entities {
                if let Some(config_id) = &entity.config_id
                    && let Some(cfg) = self.entity_configs.get(config_id)
                {
                    entity.description = cfg.description.clone();
                }
            }
            incoming.extend(dungeon_entities);
        }

        {
            let mut entities = self.active_entities.write().await;
            entities.retain(|_, e| {
                e.config_id.is_none()
                    || !removed
                        .contains(&(e.location.world_id.clone(), e.location.dungeon_id.clone()))
            });
            for entity in incoming {
                entities.insert(entity.id, entity);
            }
        }

        *self.active_dungeons.write().await = new_active_dungeons;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn load_without_config_dir_uses_defaults() {
        let state = GameState::load(None).unwrap();
        assert_eq!(state.attribute_config.attributes.len(), 10);
    }

    #[test]
    fn load_with_dir_missing_file_uses_defaults() {
        let dir = TempDir::new().unwrap();
        let state = GameState::load(Some(dir.path())).unwrap();
        assert_eq!(state.attribute_config.attributes.len(), 10);
    }

    #[test]
    fn load_with_attributes_toml_reads_file() {
        let dir = TempDir::new().unwrap();
        let toml_path = dir.path().join("attributes.toml");
        let mut file = std::fs::File::create(&toml_path).unwrap();
        file.write_all(
            br#"
[[attributes]]
id = "custom_hp"
title = "Custom HP"
description = "Custom hit points."
min_value = 0
max_value = 50
attribute_type = "hp"
"#,
        )
        .unwrap();

        let state = GameState::load(Some(dir.path())).unwrap();
        assert_eq!(state.attribute_config.attributes.len(), 1);
        assert_eq!(state.attribute_config.attributes[0].id, "custom_hp");
    }

    #[test]
    fn load_with_mud_toml_reads_file() {
        let dir = TempDir::new().unwrap();
        let toml_path = dir.path().join("mud.toml");
        let mut file = std::fs::File::create(&toml_path).unwrap();
        file.write_all(
            br#"
[game_loop]
tick_rate_ms = 500
max_engage_ms = 15000
world_update_ms = 300000

[spawn]
world_id = "default"
dungeon_id = "default"
room_id = "default"
"#,
        )
        .unwrap();

        let state = GameState::load(Some(dir.path())).unwrap();
        assert_eq!(state.mud_config.game_loop.tick_rate_ms, 500);
        assert_eq!(state.mud_config.game_loop.max_engage_ms, 15000);
        assert_eq!(state.mud_config.game_loop.world_update_ms, 300000);
    }

    #[test]
    fn load_without_mud_toml_uses_defaults() {
        let state = GameState::load(None).unwrap();
        assert_eq!(state.mud_config.game_loop.tick_rate_ms, 1000);
        assert_eq!(state.mud_config.game_loop.max_engage_ms, 30_000);
        assert_eq!(state.mud_config.game_loop.world_update_ms, 600_000);
    }

    #[tokio::test]
    async fn load_initializes_empty_entities() {
        let state = GameState::load(None).unwrap();
        let entities = state.active_entities.read().await;
        assert!(entities.is_empty());
    }

    #[tokio::test]
    async fn load_initializes_empty_dungeons() {
        let state = GameState::load(None).unwrap();
        let dungeons = state.active_dungeons.read().await;
        assert!(dungeons.is_empty());
    }
}
