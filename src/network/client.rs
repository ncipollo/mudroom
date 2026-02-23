use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use tokio::sync::mpsc;

use super::event::NetworkEvent;

pub async fn connect_sse(
    url: String,
    tx: mpsc::Sender<NetworkEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client.get(format!("{url}/events")).send().await?;
    let mut stream = response.bytes_stream().eventsource();

    while let Some(event) = stream.next().await {
        match event {
            Ok(ev) => {
                if let Ok(network_event) = serde_json::from_str::<NetworkEvent>(&ev.data)
                    && tx.send(network_event).await.is_err()
                {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    Ok(())
}

pub async fn send_ping(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    client.post(format!("{url}/ping")).send().await?;
    Ok(())
}
