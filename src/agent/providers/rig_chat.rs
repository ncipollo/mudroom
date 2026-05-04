use rig::client::CompletionClient;
use rig::completion::Chat;

use crate::agent::error::AgentError;
use crate::game::entity_ai::AgentMessage;

use super::history_to_rig;

pub async fn run_agent_chat<C>(
    client: C,
    model: &str,
    instructions: &str,
    prompt: &str,
    history: &[AgentMessage],
    tools: Vec<Box<dyn rig::tool::ToolDyn>>,
) -> Result<String, AgentError>
where
    C: CompletionClient,
    C::CompletionModel: 'static,
{
    let agent = client
        .agent(model)
        .preamble(instructions)
        .tools(tools)
        .build();
    let rig_history = history_to_rig(history);
    agent
        .chat(prompt, rig_history)
        .await
        .map_err(|e| AgentError::Provider(e.to_string()))
}
