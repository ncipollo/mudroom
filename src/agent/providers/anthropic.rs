use rig::client::ProviderClient;
use rig::providers::anthropic;

use crate::agent::entity_ai::AgentMessage;
use crate::agent::error::AgentError;
use crate::agent::provider::{AgentProvider, BoxFuture};

use super::rig_chat::run_agent_chat;

pub struct AnthropicProvider {
    pub api_key: Option<String>,
    pub model: String,
}

impl AgentProvider for AnthropicProvider {
    fn chat<'a>(
        &'a self,
        instructions: &'a str,
        prompt: &'a str,
        history: &'a [AgentMessage],
        tools: Vec<Box<dyn rig::tool::ToolDyn>>,
    ) -> BoxFuture<'a, Result<String, AgentError>> {
        Box::pin(async move {
            let client = match &self.api_key {
                Some(key) => anthropic::Client::from_val(key.clone()),
                None => anthropic::Client::from_env(),
            };
            run_agent_chat(client, &self.model, instructions, prompt, history, tools).await
        })
    }
}
