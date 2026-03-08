use serde::{Deserialize, Serialize};

use crate::game::component::interaction::direction::Direction;
use crate::game::map::Navigation;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Movement {
    TryDirection(Direction),
    Warp(Navigation),
}
