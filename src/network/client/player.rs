use serde_json::json;

use crate::network::event::{PlayerInfo, PlayerListResponse};

pub async fn list_players(
    url: &str,
    client_id: &str,
) -> Result<Vec<PlayerInfo>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let body = json!({ "client_id": client_id });
    let resp = client
        .post(format!("{url}/players/list"))
        .json(&body)
        .send()
        .await?
        .json::<PlayerListResponse>()
        .await?;
    Ok(resp.players)
}

pub async fn create_player(
    url: &str,
    client_id: &str,
    name: &str,
) -> Result<PlayerInfo, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let body = json!({ "client_id": client_id, "name": name });
    let resp = client
        .post(format!("{url}/players/create"))
        .json(&body)
        .send()
        .await?
        .json::<PlayerInfo>()
        .await?;
    Ok(resp)
}

pub async fn select_player(
    url: &str,
    client_id: &str,
    player_id: i64,
) -> Result<PlayerInfo, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let body = json!({ "client_id": client_id, "player_id": player_id });
    let resp = client
        .post(format!("{url}/players/select"))
        .json(&body)
        .send()
        .await?
        .json::<PlayerInfo>()
        .await?;
    Ok(resp)
}
