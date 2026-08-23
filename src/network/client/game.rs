use std::sync::OnceLock;

use crate::game::Interaction;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Returns a shared client so interaction sends reuse a pooled connection
/// instead of paying fresh TCP (and possibly DNS) setup on every request.
fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(reqwest::Client::new)
}

pub async fn send_interaction(
    url: &str,
    client_id: &str,
    interaction: &Interaction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let body = serde_json::json!({
        "client_id": client_id,
        "interaction": interaction,
    });
    client()
        .post(format!("{url}/interactions"))
        .json(&body)
        .send()
        .await?;
    Ok(())
}
