use rig::client::CompletionClient;
use rig::completion::Chat;

use crate::agent::entity_ai::AgentMessage;
use crate::agent::error::AgentError;

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
    run_agent_chat_with_params(client, model, instructions, prompt, history, tools, None).await
}

pub async fn run_agent_chat_with_params<C>(
    client: C,
    model: &str,
    instructions: &str,
    prompt: &str,
    history: &[AgentMessage],
    tools: Vec<Box<dyn rig::tool::ToolDyn>>,
    additional_params: Option<serde_json::Value>,
) -> Result<String, AgentError>
where
    C: CompletionClient,
    C::CompletionModel: 'static,
{
    let mut builder = client.agent(model).preamble(instructions).tools(tools);
    if let Some(params) = additional_params {
        builder = builder.additional_params(params);
    }
    let agent = builder.build();
    let rig_history = history_to_rig(history);
    agent
        .chat(prompt, rig_history)
        .await
        .map_err(|e| AgentError::Provider(e.to_string()))
}
