use crate::network::event::ServerInfoResponse;

pub async fn get_server_info(url: &str) -> Result<ServerInfoResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{url}/server/info"))
        .send()
        .await?
        .json::<ServerInfoResponse>()
        .await?;
    Ok(resp)
}
