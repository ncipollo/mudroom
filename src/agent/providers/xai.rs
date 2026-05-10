use rig::client::ProviderClient;
use rig::providers::xai;

use crate::agent::entity_ai::AgentMessage;
use crate::agent::error::AgentError;
use crate::agent::provider::{AgentProvider, BoxFuture};

use super::rig_chat::run_agent_chat;

pub struct XaiProvider {
    pub api_key: Option<String>,
    pub model: String,
}

impl AgentProvider for XaiProvider {
    fn chat<'a>(
        &'a self,
        instructions: &'a str,
        prompt: &'a str,
        history: &'a [AgentMessage],
        tools: Vec<Box<dyn rig::tool::ToolDyn>>,
    ) -> BoxFuture<'a, Result<String, AgentError>> {
        Box::pin(async move {
            let client = match &self.api_key {
                Some(key) => xai::Client::from_val(key.clone()),
                None => xai::Client::from_env(),
            };
            run_agent_chat(client, &self.model, instructions, prompt, history, tools).await
        })
    }
}
