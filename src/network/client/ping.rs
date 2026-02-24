use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

pub async fn run_ping_loop(
    url: String,
    client_id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let interval = Duration::from_secs(10);
    loop {
        sleep(interval).await;
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
