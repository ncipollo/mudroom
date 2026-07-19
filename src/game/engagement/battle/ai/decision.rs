use std::collections::HashMap;

use crate::game::component::{Ability, Attribute};

pub type AiAction = (i64, Ability, i64, HashMap<String, Attribute>);

pub enum AiDecision {
    Action(Box<AiAction>),
    Skip(i64),
}
