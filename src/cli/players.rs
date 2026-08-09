use std::collections::HashMap;

use crate::game;
use crate::game::component::{Ability, Attribute};
use crate::game::config::class_config::ClassConfig;
use crate::paths;
use crate::persistence::{self, ability_repo, character_repo, player_repo};

use super::PlayersCommands;

pub async fn run(command: PlayersCommands) -> Result<(), Box<dyn std::error::Error>> {
    paths::create_session_base_dirs().await?;
    match command {
        PlayersCommands::List { name } => {
            let db = open_players_db(name).await?;
            let players = player_repo::find_all(db.pool()).await?;
            for p in players {
                println!("{}\t{}\t{}", p.id, p.client_id, p.name);
            }
        }
        PlayersCommands::Rm { player, name } => {
            let db = open_players_db(name).await?;
            let p = player_repo::find_by_name(db.pool(), &player)
                .await?
                .ok_or_else(|| format!("player '{player}' not found"))?;
            character_repo::delete(db.pool(), p.entity_id).await?;
            println!("Player '{player}' removed.");
        }
        PlayersCommands::Reset {
            player,
            class,
            name,
        } => {
            let db = open_players_db(name).await?;
            let game_state = game::GameState::load(paths::find_config_dir().as_deref())?;
            let p = player_repo::find_by_name(db.pool(), &player)
                .await?
                .ok_or_else(|| format!("player '{player}' not found"))?;
            let class_config = game_state
                .classes
                .get(&class)
                .ok_or_else(|| format!("class '{class}' not found"))?;
            let abilities = resolve_abilities(&game_state, class_config)?;
            let attributes = build_class_attributes(class_config);
            character_repo::update_attributes(db.pool(), p.entity_id, &attributes).await?;
            for ability in &abilities {
                ability_repo::upsert(db.pool(), ability).await?;
            }
            let ids: Vec<&str> = abilities.iter().map(|a| a.id.as_str()).collect();
            ability_repo::set_character_abilities(db.pool(), p.entity_id, &ids).await?;
            println!("Player '{player}' reset to class '{class}'.");
        }
    }
    Ok(())
}

async fn open_players_db(
    name: Option<String>,
) -> Result<persistence::Database, Box<dyn std::error::Error>> {
    let server_key = match name {
        Some(n) => n,
        None => paths::find_last_server_key()?
            .ok_or("no server found — start the server at least once first")?,
    };
    Ok(persistence::Database::connect(&server_key).await?)
}

fn build_class_attributes(class: &ClassConfig) -> HashMap<String, Attribute> {
    class
        .attributes
        .iter()
        .map(|sa| {
            (
                sa.definition_id.clone(),
                Attribute::new(
                    sa.definition_id.clone(),
                    sa.min_value,
                    sa.max_value,
                    sa.current_value,
                ),
            )
        })
        .collect()
}

fn resolve_abilities(
    game_state: &game::GameState,
    class: &ClassConfig,
) -> Result<Vec<Ability>, Box<dyn std::error::Error>> {
    class
        .innate_abilities
        .iter()
        .map(|r| {
            game_state
                .abilities
                .get(&r.0)
                .cloned()
                .ok_or_else(|| format!("ability '{}' not found in config", r.0).into())
        })
        .collect()
}
