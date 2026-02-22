pub mod cli;

use cli::{Cli, Commands};

pub fn run(cli: Cli) -> String {
    match cli.command {
        Some(Commands::Server) => "server mode".to_string(),
        None | Some(Commands::Client) => "client mode".to_string(),
    }
}
