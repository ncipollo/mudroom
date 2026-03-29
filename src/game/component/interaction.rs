pub mod direction;
pub mod movement;

pub use direction::Direction;
pub use movement::Movement;

use serde::{Deserialize, Serialize};

use crate::game::engagement::TurnAction;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Interaction {
    Look,
    Movement(Movement),
    EngagementAction(TurnAction),
}
