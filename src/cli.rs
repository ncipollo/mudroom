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
    Client {
        /// Direct server URL (skips discovery)
        #[arg(long)]
        url: Option<String>,
    },
    /// Launch in server mode
    Server {
        /// Optional name for this server
        #[arg(long)]
        name: Option<String>,
    },
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
        assert_eq!(cli.command, Some(Commands::Client { url: None }));
    }

    #[test]
    fn client_subcommand_with_url_parses() {
        let cli = parse(&["mudroom", "client", "--url", "http://localhost:8080"]);
        assert_eq!(
            cli.command,
            Some(Commands::Client {
                url: Some("http://localhost:8080".to_string())
            })
        );
    }

    #[test]
    fn server_subcommand_parses() {
        let cli = parse(&["mudroom", "server"]);
        assert_eq!(cli.command, Some(Commands::Server { name: None }));
    }

    #[test]
    fn server_subcommand_with_name_parses() {
        let cli = parse(&["mudroom", "server", "--name", "myserver"]);
        assert_eq!(
            cli.command,
            Some(Commands::Server {
                name: Some("myserver".to_string())
            })
        );
    }
}
