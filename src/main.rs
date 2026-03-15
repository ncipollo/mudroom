use clap::Parser;
use mudroom::{cli::Cli, run_cli};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = run_cli(cli).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
