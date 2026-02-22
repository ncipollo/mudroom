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
    fn no_command_gives_client_mode() {
        assert_eq!(crate::run(parse(&["mudroom"])), "client mode");
    }

    #[test]
    fn client_command_gives_client_mode() {
        assert_eq!(crate::run(parse(&["mudroom", "client"])), "client mode");
    }

    #[test]
    fn server_command_gives_server_mode() {
        assert_eq!(crate::run(parse(&["mudroom", "server"])), "server mode");
    }
}
