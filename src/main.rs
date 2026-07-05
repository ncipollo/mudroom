use clap::Parser;
use mudroom::cli::{self, Cli};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = cli::router().route(cli).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
