use serde::{Deserialize, Serialize};

use super::Navigation;
use crate::game::Description;
use crate::game::Entity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: i64,
    pub description: Description,
    pub entities: Vec<Entity>,
    pub north: Option<Navigation>,
    pub south: Option<Navigation>,
    pub east: Option<Navigation>,
    pub west: Option<Navigation>,
}

impl Room {
    pub fn new(id: i64, description: Description) -> Self {
        Self {
            id,
            description,
            entities: Vec::new(),
            north: None,
            south: None,
            east: None,
            west: None,
        }
    }
}
