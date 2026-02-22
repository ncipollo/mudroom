use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "mudroom", about = "mudroom application")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// Launch in client mode
    Client,
    /// Launch in server mode
    Server,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Cli {
        Cli::parse_from(args)
    }

    #[test]
    fn no_command_defaults_to_none() {
        let cli = parse(&["mudroom"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn client_subcommand_parses() {
        let cli = parse(&["mudroom", "client"]);
        assert_eq!(cli.command, Some(Commands::Client));
    }

    #[test]
    fn server_subcommand_parses() {
        let cli = parse(&["mudroom", "server"]);
        assert_eq!(cli.command, Some(Commands::Server));
    }
}
