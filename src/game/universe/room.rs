use serde::{Deserialize, Serialize};

use super::Navigation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub description: String,
    pub north: Option<Navigation>,
    pub south: Option<Navigation>,
    pub east: Option<Navigation>,
    pub west: Option<Navigation>,
}

impl Room {
    pub fn new(description: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            description,
            north: None,
            south: None,
            east: None,
            west: None,
        }
    }
}
