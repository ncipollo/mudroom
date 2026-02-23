use serde::Deserialize;
use tokio::net::UdpSocket;
use tokio::time::{Duration, sleep};

const MAGIC: &[u8] = b"mdrm";
const DISCOVERY_PORT: u16 = 7878;
const BROADCAST_ADDR: &str = "255.255.255.255";

#[derive(Debug, Clone, Deserialize)]
pub struct DiscoveredServer {
    pub host: String,
    pub port: u16,
    pub name: String,
}

impl DiscoveredServer {
    pub fn url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

pub async fn discover(
    timeout_ms: u64,
) -> Result<Vec<DiscoveredServer>, Box<dyn std::error::Error + Send + Sync>> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.set_broadcast(true)?;

    let broadcast_addr = format!("{BROADCAST_ADDR}:{DISCOVERY_PORT}");
    socket.send_to(MAGIC, &broadcast_addr).await?;

    let mut servers = Vec::new();
    let mut buf = [0u8; 512];
    let deadline = sleep(Duration::from_millis(timeout_ms));
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            _ = &mut deadline => break,
            result = socket.recv_from(&mut buf) => {
                if let Ok((len, _)) = result
                    && let Ok(text) = std::str::from_utf8(&buf[..len])
                    && let Ok(server) = serde_json::from_str::<DiscoveredServer>(text)
                {
                    servers.push(server);
                }
            }
        }
    }

    Ok(servers)
}
