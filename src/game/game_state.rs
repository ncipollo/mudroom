use std::collections::HashMap;
use std::path::Path;

use tokio::sync::RwLock;
use tokio::sync::broadcast;

use crate::game::config::{AttributeConfig, MudConfig};
use crate::game::entity::Entity;
use crate::game::mailbox::Mailboxes;
use crate::game::messaging::PlayerMessage;
use crate::game::player::Player;

pub struct GameState {
    pub attribute_config: AttributeConfig,
    pub mud_config: MudConfig,
    pub active_entities: RwLock<HashMap<i64, Entity>>,
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

        let (message_tx, _) = broadcast::channel::<PlayerMessage>(64);

        Ok(Self {
            attribute_config,
            mud_config,
            active_entities: RwLock::new(HashMap::new()),
            mailboxes: Mailboxes::new(),
            active_players: RwLock::new(HashMap::new()),
            message_tx,
        })
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
tick_rate = 500
max_turn_ticks = 15
world_update_ticks = 300

[spawn]
world_id = "default"
dungeon_id = "default"
room_id = "default"
"#,
        )
        .unwrap();

        let state = GameState::load(Some(dir.path())).unwrap();
        assert_eq!(state.mud_config.game_loop.tick_rate, 500);
        assert_eq!(state.mud_config.game_loop.max_turn_ticks, 15);
        assert_eq!(state.mud_config.game_loop.world_update_ticks, 300);
    }

    #[test]
    fn load_without_mud_toml_uses_defaults() {
        let state = GameState::load(None).unwrap();
        assert_eq!(state.mud_config.game_loop.tick_rate, 1000);
        assert_eq!(state.mud_config.game_loop.max_turn_ticks, 30);
        assert_eq!(state.mud_config.game_loop.world_update_ticks, 600);
    }

    #[tokio::test]
    async fn load_initializes_empty_entities() {
        let state = GameState::load(None).unwrap();
        let entities = state.active_entities.read().await;
        assert!(entities.is_empty());
    }
}
