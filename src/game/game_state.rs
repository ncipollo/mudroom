use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tokio::sync::broadcast;

use crate::game::config::{
    AttributeConfig, ClassConfig, EntityConfig, FactionConfig, MudConfig, ResourceConfig,
    load_classes, load_entity_configs,
};
use crate::game::engagement::Engagements;
use crate::game::entity::Entity;
use crate::game::mailbox::Mailboxes;
use crate::game::messaging::PlayerMessage;
use crate::game::player::Player;
use crate::persistence::PersistenceError;

pub struct PendingActivation {
    pub entity: Entity,
    pub player: Player,
    pub client_id: String,
}

mod entity_sync;

pub struct GameState {
    pub config_path: Option<PathBuf>,
    pub reload_pending: AtomicBool,
    pub attribute_config: AttributeConfig,
    pub faction_config: FactionConfig,
    pub resource_config: ResourceConfig,
    pub mud_config: MudConfig,
    pub entity_configs: HashMap<String, EntityConfig>,
    pub classes: HashMap<String, ClassConfig>,
    pub active_entities: RwLock<HashMap<i64, Entity>>,
    pub active_dungeons: RwLock<HashSet<(String, String)>>,
    pub engagements: Engagements,
    pub mailboxes: Mailboxes,
    pub active_players: RwLock<HashMap<String, Player>>,
    pub pending_activations: RwLock<Vec<PendingActivation>>,
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

        let faction_config = if let Some(dir) = config_dir {
            let path = dir.join("factions.toml");
            if path.exists() {
                FactionConfig::load(&path)?
            } else {
                FactionConfig::default_config()
            }
        } else {
            FactionConfig::default_config()
        };

        let resource_config = if let Some(dir) = config_dir {
            let path = dir.join("resources.toml");
            if path.exists() {
                ResourceConfig::load(&path)?
            } else {
                ResourceConfig::default_config()
            }
        } else {
            ResourceConfig::default_config()
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

        let classes = if let Some(dir) = config_dir {
            load_classes(dir).unwrap_or_default()
        } else {
            HashMap::new()
        };

        let (message_tx, _) = broadcast::channel::<PlayerMessage>(512);

        Ok(Self {
            config_path: config_dir.map(Path::to_path_buf),
            reload_pending: AtomicBool::new(false),
            attribute_config,
            faction_config,
            resource_config,
            mud_config,
            entity_configs,
            classes,
            active_entities: RwLock::new(HashMap::new()),
            active_dungeons: RwLock::new(HashSet::new()),
            engagements: Engagements::new(),
            mailboxes: Mailboxes::new(),
            active_players: RwLock::new(HashMap::new()),
            pending_activations: RwLock::new(Vec::new()),
            message_tx,
        })
    }

    pub async fn sync_active_entities(&self, pool: &SqlitePool) -> Result<(), PersistenceError> {
        entity_sync::sync(self, pool).await
    }

    pub async fn push_pending_activation(&self, entity: Entity, player: Player, client_id: String) {
        self.pending_activations
            .write()
            .await
            .push(PendingActivation {
                entity,
                player,
                client_id,
            });
    }

    pub async fn drain_pending_activations(&self) -> Vec<PendingActivation> {
        let mut pending = self.pending_activations.write().await;
        std::mem::take(&mut *pending)
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
attribute_category = "life"
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
        assert_eq!(state.mud_config.game_loop.max_engage_ms, 300_000);
        assert_eq!(state.mud_config.game_loop.world_update_ms, 600_000);
    }

    #[test]
    fn load_without_config_dir_uses_default_factions() {
        let state = GameState::load(None).unwrap();
        assert_eq!(state.faction_config.factions.len(), 2);
        let ids: Vec<&str> = state
            .faction_config
            .factions
            .iter()
            .map(|f| f.id.as_str())
            .collect();
        assert!(ids.contains(&"player"));
        assert!(ids.contains(&"enemy"));
    }

    #[test]
    fn load_with_factions_toml_reads_file() {
        let dir = TempDir::new().unwrap();
        let toml_path = dir.path().join("factions.toml");
        let mut file = std::fs::File::create(&toml_path).unwrap();
        file.write_all(
            br#"
[[factions]]
id = "guard"
name = "Guard"
description = "City guards."
"#,
        )
        .unwrap();

        let state = GameState::load(Some(dir.path())).unwrap();
        assert_eq!(state.faction_config.factions.len(), 1);
        assert_eq!(state.faction_config.factions[0].id, "guard");
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
