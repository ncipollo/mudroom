use serde::{Deserialize, Serialize};

use super::Navigation;
use crate::game::Description;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub description: Description,
    pub north: Option<Navigation>,
    pub south: Option<Navigation>,
    pub east: Option<Navigation>,
    pub west: Option<Navigation>,
}

impl Room {
    pub fn new(id: String, description: Description) -> Self {
        Self {
            id,
            description,
            north: None,
            south: None,
            east: None,
            west: None,
        }
    }
}
