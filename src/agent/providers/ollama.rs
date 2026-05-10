use rig::client::Nothing;
use rig::providers::ollama;

use crate::agent::entity_ai::AgentMessage;
use crate::agent::error::AgentError;
use crate::agent::provider::{AgentProvider, BoxFuture};

use super::rig_chat::run_agent_chat;

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
            run_agent_chat(client, &self.model, instructions, prompt, history, tools).await
        })
    }
}
