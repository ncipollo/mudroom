use serde_json::json;

use crate::network::event::SessionStartResponse;

pub async fn start_session(
    url: &str,
    connection_key: String,
) -> Result<SessionStartResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let body = json!({ "client_id": connection_key });
    let resp = client
        .post(format!("{url}/session/start"))
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(msg.into());
    }
    let session_resp = resp.json::<SessionStartResponse>().await?;
    Ok(session_resp)
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
