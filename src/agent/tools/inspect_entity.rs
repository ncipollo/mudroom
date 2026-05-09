use std::sync::Arc;

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::error::AgentError;
use crate::game::GameState;

#[derive(Deserialize, Serialize)]
pub struct InspectArgs {
    pub entity_id: Option<i64>,
    pub name: Option<String>,
}

pub struct InspectEntity {
    pub game_state: Arc<GameState>,
    pub npc_entity_id: i64,
}

impl Tool for InspectEntity {
    const NAME: &'static str = "inspect_entity";
    type Error = AgentError;
    type Args = InspectArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "inspect_entity".to_string(),
            description:
                "Inspect another entity in the current room. Returns their description and location."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "entity_id": {
                        "type": "integer",
                        "description": "The numeric ID of the entity to inspect"
                    },
                    "name": {
                        "type": "string",
                        "description": "A name or description fragment to match against entities in the room"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let entities = self.game_state.active_entities.read().await;

        let npc = entities
            .get(&self.npc_entity_id)
            .ok_or_else(|| AgentError::Tool("NPC not found".to_string()))?;
        let location = npc.location.clone();

        let target = entities.values().find(|e| {
            if e.id == self.npc_entity_id {
                return false;
            }
            if e.location != location {
                return false;
            }
            if let Some(id) = args.entity_id
                && e.id == id
            {
                return true;
            }
            if let Some(ref name) = args.name
                && e.description
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&name.to_lowercase())
            {
                return true;
            }
            false
        });

        match target {
            Some(entity) => Ok(json!({
                "entity_id": entity.id,
                "description": entity.description.as_deref().unwrap_or("unknown"),
                "location": {
                    "world_id": entity.location.world_id,
                    "dungeon_id": entity.location.dungeon_id,
                    "room_id": entity.location.room_id,
                }
            })),
            None => Err(AgentError::Tool("Entity not found in room".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::agent::tools::inspect_entity::{InspectArgs, InspectEntity};
    use crate::game::Location;
    use crate::game::entity::{Entity, EntityType};
    use crate::game::game_state::GameState;
    use rig::tool::Tool;

    fn room_location() -> Location {
        Location {
            world_id: "default".to_string(),
            dungeon_id: "default".to_string(),
            room_id: "default".to_string(),
        }
    }

    async fn state_with_entities(entities: Vec<Entity>) -> Arc<GameState> {
        let state = Arc::new(GameState::load(None).unwrap());
        {
            let mut map = state.active_entities.write().await;
            for e in entities {
                map.insert(e.id, e);
            }
        }
        state
    }

    #[tokio::test]
    async fn finds_entity_by_id() {
        let npc = Entity::new(1, EntityType::Character, room_location());
        let mut player = Entity::new(2, EntityType::Character, room_location());
        player.description = Some("the player".to_string());
        let state = state_with_entities(vec![npc, player]).await;

        let tool = InspectEntity {
            game_state: state,
            npc_entity_id: 1,
        };
        let result = tool
            .call(InspectArgs {
                entity_id: Some(2),
                name: None,
            })
            .await
            .unwrap();

        assert_eq!(result["entity_id"], 2);
        assert_eq!(result["description"], "the player");
    }

    #[tokio::test]
    async fn finds_entity_by_name() {
        let npc = Entity::new(1, EntityType::Character, room_location());
        let mut player = Entity::new(2, EntityType::Character, room_location());
        player.description = Some("grizzled merchant".to_string());
        let state = state_with_entities(vec![npc, player]).await;

        let tool = InspectEntity {
            game_state: state,
            npc_entity_id: 1,
        };
        let result = tool
            .call(InspectArgs {
                entity_id: None,
                name: Some("merchant".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(result["entity_id"], 2);
    }

    #[tokio::test]
    async fn errors_when_not_in_room() {
        let npc = Entity::new(1, EntityType::Character, room_location());
        let mut other = Entity::new(2, EntityType::Character, room_location());
        other.location.room_id = "other_room".to_string();
        other.description = Some("stranger".to_string());
        let state = state_with_entities(vec![npc, other]).await;

        let tool = InspectEntity {
            game_state: state,
            npc_entity_id: 1,
        };
        let result = tool
            .call(InspectArgs {
                entity_id: None,
                name: Some("stranger".to_string()),
            })
            .await;

        assert!(result.is_err());
    }
}
