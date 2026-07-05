use super::{Cli, Commands, client, completions, players, server};

pub struct CliRouter;

impl CliRouter {
    pub async fn route(self, cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
        match cli.command {
            Some(Commands::Server {
                name,
                config,
                reload_maps,
            }) => server::run(name, config, reload_maps).await,
            Some(Commands::Client { url, debug }) => client::run(url, debug).await,
            Some(Commands::Players { command }) => players::run(command).await,
            Some(Commands::Completions { shell }) => {
                completions::run(shell);
                Ok(())
            }
            None => client::run(None, false).await,
        }
    }
}

pub fn router() -> CliRouter {
    CliRouter
}
