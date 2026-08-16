use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tokio::sync::broadcast;

use crate::game::component::{Ability, ItemDefinition};
use crate::game::config::{
    AttributeConfig, CharacterConfig, ClassConfig, FactionConfig, InventoryConfig, MudConfig,
    ResourceConfig, ThemeConfig, load_character_configs, load_classes, load_themes,
};
use crate::game::engagement::Engagements;
use crate::game::entity::character::Character;
use crate::game::mailbox::Mailboxes;
use crate::game::messaging::PlayerMessage;
use crate::game::player::Player;
use crate::persistence::PersistenceError;

pub struct PendingActivation {
    pub character: Character,
    pub player: Player,
    pub client_id: String,
}

mod character_sync;
mod definition_cache;

pub struct GameState {
    pub config_path: Option<PathBuf>,
    pub reload_pending: AtomicBool,
    pub attribute_config: AttributeConfig,
    pub faction_config: FactionConfig,
    pub resource_config: ResourceConfig,
    pub inventory_config: InventoryConfig,
    pub mud_config: MudConfig,
    pub abilities: RwLock<HashMap<String, Ability>>,
    pub item_definitions: RwLock<HashMap<String, ItemDefinition>>,
    pub character_configs: HashMap<String, CharacterConfig>,
    pub classes: HashMap<String, ClassConfig>,
    pub themes: HashMap<String, ThemeConfig>,
    pub active_characters: RwLock<HashMap<i64, Character>>,
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
        let attribute_config = load_file_config(
            config_dir,
            "attributes.toml",
            AttributeConfig::load,
            AttributeConfig::default_config,
        )?;
        let faction_config = load_file_config(
            config_dir,
            "factions.toml",
            FactionConfig::load,
            FactionConfig::default_config,
        )?;
        let resource_config = load_file_config(
            config_dir,
            "resources.toml",
            ResourceConfig::load,
            ResourceConfig::default_config,
        )?;
        let inventory_config = load_file_config(
            config_dir,
            "inventory.toml",
            InventoryConfig::load,
            InventoryConfig::default_config,
        )?;
        let mud_config = load_file_config(
            config_dir,
            "mud.toml",
            MudConfig::load,
            MudConfig::default_config,
        )?;

        let character_configs = load_dir_config(config_dir, load_character_configs);
        let classes = load_dir_config(config_dir, load_classes);
        let themes = load_dir_config(config_dir, load_themes);

        let (message_tx, _) = broadcast::channel::<PlayerMessage>(512);

        Ok(Self {
            config_path: config_dir.map(Path::to_path_buf),
            reload_pending: AtomicBool::new(false),
            attribute_config,
            faction_config,
            resource_config,
            inventory_config,
            mud_config,
            abilities: RwLock::new(HashMap::new()),
            item_definitions: RwLock::new(HashMap::new()),
            character_configs,
            classes,
            themes,
            active_characters: RwLock::new(HashMap::new()),
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

    pub async fn sync_active_characters(&self, pool: &SqlitePool) -> Result<(), PersistenceError> {
        character_sync::sync(self, pool).await
    }

    /// Refreshes the `abilities` and `item_definitions` caches from the database, which is the
    /// durable source of truth kept current by `sync_universe_config`.
    pub async fn refresh_definition_caches(
        &self,
        pool: &SqlitePool,
    ) -> Result<(), PersistenceError> {
        definition_cache::refresh(self, pool).await
    }

    pub async fn push_pending_activation(
        &self,
        character: Character,
        player: Player,
        client_id: String,
    ) {
        self.pending_activations
            .write()
            .await
            .push(PendingActivation {
                character,
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

/// Loads a single-file config from `config_dir/<filename>` when the dir and file both exist,
/// falling back to `default` otherwise.
fn load_file_config<T>(
    config_dir: Option<&Path>,
    filename: &str,
    load: impl Fn(&Path) -> Result<T, Box<dyn std::error::Error>>,
    default: impl Fn() -> T,
) -> Result<T, Box<dyn std::error::Error>> {
    let Some(dir) = config_dir else {
        return Ok(default());
    };
    let path = dir.join(filename);
    if path.exists() {
        load(&path)
    } else {
        Ok(default())
    }
}

/// Loads a directory-based config map via `load`, defaulting to an empty map when `config_dir`
/// is absent or the load fails.
fn load_dir_config<T>(
    config_dir: Option<&Path>,
    load: impl Fn(&Path) -> Result<HashMap<String, T>, Box<dyn std::error::Error>>,
) -> HashMap<String, T> {
    config_dir
        .map(|dir| load(dir).unwrap_or_default())
        .unwrap_or_default()
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

    #[test]
    fn load_without_config_dir_uses_default_inventory_config() {
        let state = GameState::load(None).unwrap();
        assert_eq!(state.inventory_config.inventories.len(), 1);
        assert!(state.inventory_config.get("inventory").is_some());
    }

    #[test]
    fn load_with_inventory_toml_reads_file() {
        let dir = TempDir::new().unwrap();
        let toml_path = dir.path().join("inventory.toml");
        let mut file = std::fs::File::create(&toml_path).unwrap();
        file.write_all(
            br#"
[[inventories]]
id = "inventory"
bag_size = 5

[[inventories.equipment_slots]]
name = "trinket"
item_types = ["trinket"]
"#,
        )
        .unwrap();

        let state = GameState::load(Some(dir.path())).unwrap();
        let def = state.inventory_config.get("inventory").unwrap();
        assert_eq!(def.bag_size, 5);
        assert_eq!(def.equipment_slots.len(), 1);
        assert_eq!(def.equipment_slots[0].name, "trinket");
    }

    #[test]
    fn load_without_config_dir_uses_empty_themes() {
        let state = GameState::load(None).unwrap();
        assert!(state.themes.is_empty());
    }

    #[test]
    fn load_with_themes_dir_reads_files() {
        let dir = TempDir::new().unwrap();
        let themes_dir = dir.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();
        let toml_path = themes_dir.join("eerie.toml");
        let mut file = std::fs::File::create(&toml_path).unwrap();
        file.write_all(
            br#"
[styles.bold]
fg = "red"
modifiers = ["bold"]
"#,
        )
        .unwrap();

        let state = GameState::load(Some(dir.path())).unwrap();
        assert_eq!(state.themes.len(), 1);
        assert!(state.themes.contains_key("eerie"));
    }

    #[tokio::test]
    async fn load_initializes_empty_characters() {
        let state = GameState::load(None).unwrap();
        let characters = state.active_characters.read().await;
        assert!(characters.is_empty());
    }

    #[tokio::test]
    async fn load_initializes_empty_dungeons() {
        let state = GameState::load(None).unwrap();
        let dungeons = state.active_dungeons.read().await;
        assert!(dungeons.is_empty());
    }
}
