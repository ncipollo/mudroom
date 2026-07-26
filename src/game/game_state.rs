use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tokio::sync::broadcast;

use crate::game::component::Ability;
use crate::game::config::{
    AttributeConfig, ClassConfig, EntityConfig, FactionConfig, MudConfig, ResourceConfig,
    load_abilities, load_classes, load_entity_configs,
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
    pub abilities: HashMap<String, Ability>,
    pub entity_configs: HashMap<String, EntityConfig>,
    pub classes: HashMap<String, ClassConfig>,
    pub active_entities: RwLock<HashMap<i64, Entity>>,
    pub active_dungeons: RwLock<HashSet<(String, String)>>,
    pub engagements: Engagements,
    pub mailboxes: Mailboxes,
    pub active_players: RwLock<HashMap<String, Player>>,
    pub pending_activations: RwLock<Vec<PendingActivation>>,
    /// Per-entity activation epoch. Bumped every time an entity is (re)activated so that a
    /// `PlayerDisconnected` interaction queued before the bump can be recognized as stale even
    /// if it lands in the mailbox after the one-time discard step in `apply_activation` already
    /// ran — see the epoch check in `game::interaction::dispatch_player_disconnected`.
    activation_epochs: RwLock<HashMap<i64, u64>>,
    next_activation_epoch: AtomicU64,
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

        let abilities = if let Some(dir) = config_dir {
            load_abilities(dir).unwrap_or_default()
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
            abilities,
            entity_configs,
            classes,
            active_entities: RwLock::new(HashMap::new()),
            active_dungeons: RwLock::new(HashSet::new()),
            engagements: Engagements::new(),
            mailboxes: Mailboxes::new(),
            active_players: RwLock::new(HashMap::new()),
            pending_activations: RwLock::new(Vec::new()),
            activation_epochs: RwLock::new(HashMap::new()),
            next_activation_epoch: AtomicU64::new(1),
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

    /// Marks `entity_id` as (re)activated as of a new epoch, returning that epoch. Any
    /// previously-captured epoch for this entity is now stale.
    pub async fn bump_activation_epoch(&self, entity_id: i64) -> u64 {
        let epoch = self.next_activation_epoch.fetch_add(1, Ordering::Relaxed);
        self.activation_epochs
            .write()
            .await
            .insert(entity_id, epoch);
        epoch
    }

    /// The entity's current activation epoch, or 0 if it has never been activated.
    pub async fn current_activation_epoch(&self, entity_id: i64) -> u64 {
        *self
            .activation_epochs
            .read()
            .await
            .get(&entity_id)
            .unwrap_or(&0)
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
