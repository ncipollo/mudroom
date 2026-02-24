use serde_json::json;

use crate::network::event::SessionStartResponse;

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
