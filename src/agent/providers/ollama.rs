use rig::client::{CompletionClient, Nothing};
use rig::completion::Chat;
use rig::providers::ollama;

use crate::agent::error::AgentError;
use crate::agent::provider::AgentProvider;
use crate::agent::provider::BoxFuture;
use crate::game::entity_ai::AgentMessage;

use super::history_to_rig;

pub struct OllamaProvider {
    pub base_url: String,
    pub model: String,
}

impl AgentProvider for OllamaProvider {
    fn chat<'a>(
        &'a self,
        instructions: &'a str,
        prompt: &'a str,
        history: &'a [AgentMessage],
        tools: Vec<Box<dyn rig::tool::ToolDyn>>,
    ) -> BoxFuture<'a, Result<String, AgentError>> {
        Box::pin(async move {
            let client = ollama::Client::builder()
                .api_key(Nothing)
                .base_url(&self.base_url)
                .build()
                .map_err(|e| AgentError::Provider(e.to_string()))?;

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
