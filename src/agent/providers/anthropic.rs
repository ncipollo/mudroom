use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Chat;
use rig::providers::anthropic;

use crate::agent::error::AgentError;
use crate::agent::provider::{AgentProvider, BoxFuture};
use crate::game::entity_ai::AgentMessage;

use super::history_to_rig;

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

            let agent = client
                .agent(&self.model)
                .preamble(instructions)
                .tools(tools)
                .build();
            let rig_history = history_to_rig(history);

            agent
                .chat(prompt, rig_history)
                .await
                .map_err(|e| AgentError::Provider(e.to_string()))
        })
    }
}
