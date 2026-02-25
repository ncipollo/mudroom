use serde_json::json;
use tokio::net::UdpSocket;

const MAGIC: &[u8] = b"mdrm";
const DISCOVERY_PORT: u16 = 7878;

pub struct DiscoveryServer {
    port: u16,
    name: Option<String>,
}

impl DiscoveryServer {
    pub fn new(http_port: u16, name: Option<String>) -> Self {
        Self {
            port: http_port,
            name,
        }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{DISCOVERY_PORT}")).await?;
        let mut buf = [0u8; 64];

        loop {
            let (len, peer) = socket.recv_from(&mut buf).await?;
            if len >= MAGIC.len() && &buf[..MAGIC.len()] == MAGIC {
                let host = peer.ip().to_string();
                let mut response = json!({ "host": host, "port": self.port });
                if let Some(ref name) = self.name {
                    response["name"] = json!(name);
                }
                let response_str = response.to_string();
                socket.send_to(response_str.as_bytes(), peer).await?;
            }
        }
    }
}
