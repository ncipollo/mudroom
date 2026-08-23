use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RespawnMode {
    OnGameReboot,
    OnRoomVisit,
    OnDungeonVisit,
    Never,
}

fn default_respawn_mode() -> RespawnMode {
    RespawnMode::OnGameReboot
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldLootConfig {
    #[serde(default = "default_respawn_mode")]
    pub respawn_mode: RespawnMode,
}

impl WorldLootConfig {
    pub fn default_config() -> Self {
        Self {
            respawn_mode: default_respawn_mode(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_on_game_reboot() {
        let config = WorldLootConfig::default_config();
        assert_eq!(config.respawn_mode, RespawnMode::OnGameReboot);
    }

    #[test]
    fn missing_respawn_mode_defaults_to_on_game_reboot() {
        let config: WorldLootConfig = toml::from_str("").unwrap();
        assert_eq!(config.respawn_mode, RespawnMode::OnGameReboot);
    }

    #[test]
    fn parses_each_respawn_mode() {
        let cases = [
            (
                "respawn_mode = \"on_game_reboot\"",
                RespawnMode::OnGameReboot,
            ),
            ("respawn_mode = \"on_room_visit\"", RespawnMode::OnRoomVisit),
            (
                "respawn_mode = \"on_dungeon_visit\"",
                RespawnMode::OnDungeonVisit,
            ),
            ("respawn_mode = \"never\"", RespawnMode::Never),
        ];
        for (toml_str, expected) in cases {
            let config: WorldLootConfig = toml::from_str(toml_str).unwrap();
            assert_eq!(config.respawn_mode, expected);
        }
    }
}
