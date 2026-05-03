use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Chat;
use rig::providers::openai;

use crate::agent::error::AgentError;
use crate::agent::provider::{AgentProvider, BoxFuture};
use crate::game::entity_ai::AgentMessage;

use super::history_to_rig;

pub struct OpenAiProvider {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
}

impl AgentProvider for OpenAiProvider {
    fn chat<'a>(
        &'a self,
        instructions: &'a str,
        prompt: &'a str,
        history: &'a [AgentMessage],
        tools: Vec<Box<dyn rig::tool::ToolDyn>>,
    ) -> BoxFuture<'a, Result<String, AgentError>> {
        Box::pin(async move {
            let client = match &self.api_key {
                None => openai::Client::from_env(),
                Some(key) => {
                    let mut builder = openai::Client::builder().api_key(key.as_str());
                    if let Some(url) = &self.base_url {
                        builder = builder.base_url(url.as_str());
                    }
                    builder
                        .build()
                        .map_err(|e| AgentError::Provider(e.to_string()))?
                }
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
