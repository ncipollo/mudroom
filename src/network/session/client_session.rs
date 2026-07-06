use serde::{Deserialize, Serialize};

use super::connection_key::ConnectionKey;
use super::error::SessionError;
use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSession {
    pub id: String,
    pub name: Option<String>,
}

impl ClientSession {
    pub fn connection_key(&self) -> ConnectionKey {
        ConnectionKey::new(&self.id)
    }

    pub async fn load(server_id: &str) -> Result<Option<Self>, SessionError> {
        let path = paths::client_session_file(server_id).map_err(|_| SessionError::NoHomeDir)?;
        if !path.exists() {
            return Ok(None);
        }
        let data = tokio::fs::read_to_string(&path).await?;
        let session: ClientSession = serde_json::from_str(&data)?;
        Ok(Some(session))
    }

    pub async fn save(&self, server_id: &str) -> Result<(), SessionError> {
        let path = paths::client_session_file(server_id).map_err(|_| SessionError::NoHomeDir)?;
        let data = serde_json::to_string_pretty(self)?;
        tokio::fs::write(&path, data).await?;
        Ok(())
    }
}
