pub mod client;
pub mod completions;
pub mod instructions;
pub mod players;
pub mod router;
pub mod server;

pub use router::router;

use clap::{Parser, Subcommand};
use clap_complete::Shell;

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
        /// Enable debug output (e.g. show ping/pong messages)
        #[arg(long)]
        debug: bool,
    },
    /// Launch in server mode
    Server {
        /// Optional name for this server
        #[arg(long)]
        name: Option<String>,
        /// Path to config directory containing game data files
        #[arg(long)]
        config: Option<String>,
        /// Force map reload from config on startup, even if maps were previously loaded
        #[arg(long)]
        reload_maps: bool,
        /// Enable debug output (e.g. show ping/pong messages)
        #[arg(long)]
        debug: bool,
    },
    /// Player administration commands
    Players {
        #[command(subcommand)]
        command: PlayersCommands,
    },
    /// Generate shell completions
    Completions {
        /// Target shell
        shell: Shell,
    },
    /// Print authoring instructions for mud config files
    #[command(alias = "info")]
    Instructions {
        #[command(subcommand)]
        topic: Option<InstructionsTopic>,
    },
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum InstructionsTopic {
    /// Mud directory layout and mud.toml configuration reference
    MudConfig,
    /// Ability config file (abilities/*.toml) reference
    Abilities,
    /// Attribute definitions (attributes.toml) reference
    Attributes,
    /// Class config file (classes/*.toml) reference
    Classes,
    /// Entity config file (entities/*.toml) reference
    Entities,
    /// Map and room config file (maps/<world>/<dungeon>/<room>.toml) reference
    Maps,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum PlayersCommands {
    /// List all players in the server database
    List {
        /// Server name — identifies which database to use (matches server --name)
        #[arg(long)]
        name: Option<String>,
    },
    /// Remove a player by name from the server database
    Rm {
        /// Player name to remove
        player: String,
        /// Server name — identifies which database to use (matches server --name)
        #[arg(long)]
        name: Option<String>,
    },
    /// Reset a player's attributes and abilities to a class baseline
    Reset {
        /// Player name to reset
        player: String,
        /// Class ID to apply
        class: String,
        /// Server name — identifies which database to use (matches server --name)
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
        assert_eq!(
            cli.command,
            Some(Commands::Client {
                url: None,
                debug: false,
            })
        );
    }

    #[test]
    fn client_subcommand_with_url_parses() {
        let cli = parse(&["mudroom", "client", "--url", "http://localhost:8080"]);
        assert_eq!(
            cli.command,
            Some(Commands::Client {
                url: Some("http://localhost:8080".to_string()),
                debug: false,
            })
        );
    }

    #[test]
    fn client_subcommand_with_debug_parses() {
        let cli = parse(&["mudroom", "client", "--debug"]);
        assert_eq!(
            cli.command,
            Some(Commands::Client {
                url: None,
                debug: true,
            })
        );
    }

    #[test]
    fn server_subcommand_parses() {
        let cli = parse(&["mudroom", "server"]);
        assert_eq!(
            cli.command,
            Some(Commands::Server {
                name: None,
                config: None,
                reload_maps: false,
                debug: false,
            })
        );
    }

    #[test]
    fn server_subcommand_with_name_parses() {
        let cli = parse(&["mudroom", "server", "--name", "myserver"]);
        assert_eq!(
            cli.command,
            Some(Commands::Server {
                name: Some("myserver".to_string()),
                config: None,
                reload_maps: false,
                debug: false,
            })
        );
    }

    #[test]
    fn server_subcommand_with_config_parses() {
        let cli = parse(&["mudroom", "server", "--config", "muds/basic"]);
        assert_eq!(
            cli.command,
            Some(Commands::Server {
                name: None,
                config: Some("muds/basic".to_string()),
                reload_maps: false,
                debug: false,
            })
        );
    }

    #[test]
    fn server_subcommand_with_debug_parses() {
        let cli = parse(&["mudroom", "server", "--debug"]);
        assert_eq!(
            cli.command,
            Some(Commands::Server {
                name: None,
                config: None,
                reload_maps: false,
                debug: true,
            })
        );
    }

    #[test]
    fn players_reset_subcommand_parses() {
        let cli = parse(&["mudroom", "players", "reset", "Alice", "warrior"]);
        assert_eq!(
            cli.command,
            Some(Commands::Players {
                command: PlayersCommands::Reset {
                    player: "Alice".to_string(),
                    class: "warrior".to_string(),
                    name: None,
                },
            })
        );
    }

    #[test]
    fn players_reset_subcommand_with_name_parses() {
        let cli = parse(&[
            "mudroom", "players", "reset", "Alice", "warrior", "--name", "myserver",
        ]);
        assert_eq!(
            cli.command,
            Some(Commands::Players {
                command: PlayersCommands::Reset {
                    player: "Alice".to_string(),
                    class: "warrior".to_string(),
                    name: Some("myserver".to_string()),
                },
            })
        );
    }

    #[test]
    fn instructions_subcommand_parses() {
        let cli = parse(&["mudroom", "instructions"]);
        assert_eq!(cli.command, Some(Commands::Instructions { topic: None }));
    }

    #[test]
    fn info_alias_parses() {
        let cli = parse(&["mudroom", "info"]);
        assert_eq!(cli.command, Some(Commands::Instructions { topic: None }));
    }

    #[test]
    fn server_subcommand_with_reload_maps_parses() {
        let cli = parse(&["mudroom", "server", "--reload-maps"]);
        assert_eq!(
            cli.command,
            Some(Commands::Server {
                name: None,
                config: None,
                reload_maps: true,
                debug: false,
            })
        );
    }
}
