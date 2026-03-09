use crate::game::Interaction;

pub async fn send_interaction(
    url: &str,
    client_id: &str,
    interaction: &Interaction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let body = serde_json::json!({
        "client_id": client_id,
        "interaction": interaction,
    });
    reqwest::Client::new()
        .post(format!("{url}/interactions"))
        .json(&body)
        .send()
        .await?;
    Ok(())
}
