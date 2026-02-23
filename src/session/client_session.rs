use serde::{Deserialize, Serialize};

use super::error::SessionError;
use crate::state::config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSession {
    pub id: String,
    pub name: Option<String>,
}

impl ClientSession {
    pub async fn load(server_id: &str) -> Result<Option<Self>, SessionError> {
        let path = config::client_session_file(server_id).map_err(|_| SessionError::NoHomeDir)?;
        if !path.exists() {
            return Ok(None);
        }
        let data = tokio::fs::read_to_string(&path).await?;
        let session: ClientSession = serde_json::from_str(&data)?;
        Ok(Some(session))
    }

    pub async fn save(&self, server_id: &str) -> Result<(), SessionError> {
        let path = config::client_session_file(server_id).map_err(|_| SessionError::NoHomeDir)?;
        let data = serde_json::to_string_pretty(self)?;
        tokio::fs::write(&path, data).await?;
        Ok(())
    }
}
