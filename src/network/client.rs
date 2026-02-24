use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde_json::json;
use tokio::sync::mpsc;

use super::event::{NetworkEvent, SessionStartResponse};

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

pub async fn start_session(
    url: &str,
    client_id: Option<String>,
) -> Result<SessionStartResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let body = json!({ "client_id": client_id });
    let resp = client
        .post(format!("{url}/session/start"))
        .json(&body)
        .send()
        .await?
        .json::<SessionStartResponse>()
        .await?;
    Ok(resp)
}

pub async fn run_ping_loop(
    url: String,
    client_id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let interval = std::time::Duration::from_secs(10);
    loop {
        tokio::time::sleep(interval).await;
        let body = json!({ "client_id": client_id });
        if client
            .post(format!("{url}/ping"))
            .json(&body)
            .send()
            .await
            .is_err()
        {
            break;
        }
    }
    Ok(())
}

pub async fn end_session(url: &str, client_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let body = json!({ "session_id": client_id });
    client
        .post(format!("{url}/session/end"))
        .json(&body)
        .send()
        .await?;
    Ok(())
}
