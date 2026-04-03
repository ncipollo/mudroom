pub mod attribute_config;
mod dialog_parser;
pub mod entity_config;
pub mod game_loop_config;
pub mod map_config;
pub mod map_loader;
pub mod mud_config;

pub use attribute_config::AttributeConfig;
pub use entity_config::{
    DialogLine, EntityConfig, EntityTypeConfig, PersonaConfig, PlayerResponse, load_entity_configs,
};
pub use game_loop_config::GameLoopConfig;
pub use map_config::load_map;
pub use map_loader::{load_entities_into_db, load_map_into_db, should_auto_load};
pub use mud_config::{MudConfig, SpawnConfig};
