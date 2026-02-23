pub mod client;
pub mod server;

pub use client::{DiscoveredServer, discover};
pub use server::DiscoveryServer;
