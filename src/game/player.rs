use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: i64,
    pub client_id: String,
    pub name: String,
    pub entity_id: i64,
}
