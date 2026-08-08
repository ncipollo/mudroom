use crate::network::event::{ThemeInfo, ThemeListResponse};

pub async fn list_themes(url: &str) -> Result<Vec<ThemeInfo>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{url}/themes/list"))
        .send()
        .await?
        .json::<ThemeListResponse>()
        .await?;
    Ok(resp.themes)
}
