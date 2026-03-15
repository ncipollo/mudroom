pub mod client;
pub mod server;

pub use client::discover;
pub use server::DiscoveryServer;

pub fn start_discovery(port: u16, session_name: Option<String>) {
    let discovery = DiscoveryServer::new(port, session_name);
    tokio::spawn(async move {
        let _ = discovery.run().await;
    });
}

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
