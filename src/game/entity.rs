pub mod character;
pub mod world_loot;

use crate::game::component::Location;
use crate::game::component::description::Description;

/// Shared identity of anything that exists in the world.
///
/// Concrete world actors (characters, and in the future items, etc.) implement
/// this trait to expose their identity fields.
pub trait Entity {
    fn id(&self) -> i64;
    fn name(&self) -> &str;
    fn description(&self) -> &Description;
    fn location(&self) -> &Location;
}
