pub mod cli;
pub mod tui;

use cli::{Cli, Commands};

pub fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Some(Commands::Server) => {
            println!("server mode");
            Ok(())
        }
        None | Some(Commands::Client) => tui::run_client(),
    }
}
