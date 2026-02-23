use serde::{Deserialize, Serialize};

use super::error::SessionError;
use crate::state::config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSession {
    pub id: String,
    pub name: Option<String>,
}

impl ServerSession {
    pub async fn load_or_create(name: Option<String>) -> Result<Self, SessionError> {
        let key = name.as_deref().unwrap_or("unnamed");
        let path = config::server_session_file(key).map_err(|_| SessionError::NoHomeDir)?;

        if path.exists() {
            let data = tokio::fs::read_to_string(&path).await?;
            let session: ServerSession = serde_json::from_str(&data)?;
            return Ok(session);
        }

        let session = ServerSession {
            id: uuid::Uuid::new_v4().to_string(),
            name,
        };
        session.save().await?;
        Ok(session)
    }

    pub async fn save(&self) -> Result<(), SessionError> {
        let key = self.name.as_deref().unwrap_or("unnamed");
        let path = config::server_session_file(key).map_err(|_| SessionError::NoHomeDir)?;
        let data = serde_json::to_string_pretty(self)?;
        tokio::fs::write(&path, data).await?;
        Ok(())
    }
}
