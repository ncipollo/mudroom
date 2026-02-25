pub mod client;
pub mod server;

pub use client::discover;
pub use server::DiscoveryServer;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredServer {
    pub host: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl DiscoveredServer {
    pub fn url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}
