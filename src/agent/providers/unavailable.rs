use crate::agent::entity_ai::AgentMessage;
use crate::agent::error::AgentError;
use crate::agent::provider::{AgentProvider, BoxFuture};

/// Stands in for a configured provider when one couldn't be built — e.g. an Ollama
/// provider whose env vars weren't set at startup. Every chat call fails the same way
/// a mid-session provider outage would, so callers degrade through one path.
pub struct UnavailableProvider;

impl AgentProvider for UnavailableProvider {
    fn chat<'a>(
        &'a self,
        _instructions: &'a str,
        _prompt: &'a str,
        _history: &'a [AgentMessage],
        _tools: Vec<Box<dyn rig::tool::ToolDyn>>,
    ) -> BoxFuture<'a, Result<String, AgentError>> {
        Box::pin(async { Err(AgentError::NotConfigured) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn chat_returns_not_configured() {
        let provider = UnavailableProvider;
        let result = provider.chat("instructions", "prompt", &[], vec![]).await;
        assert!(matches!(result, Err(AgentError::NotConfigured)));
    }
}
