use crate::tui;

pub async fn run(url: Option<String>, debug: bool) -> Result<(), Box<dyn std::error::Error>> {
    match url {
        Some(url) => tui::run_client(Some(url), debug).await,
        None => tui::run_discovery(debug).await,
    }
}
