use std::{future::Future, pin::Pin, sync::Arc};

use crate::game::entity_ai::AgentMessage;

use super::error::AgentError;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait AgentProvider: Send + Sync {
    fn chat<'a>(
        &'a self,
        instructions: &'a str,
        prompt: &'a str,
        history: &'a [AgentMessage],
        tools: Vec<Box<dyn rig::tool::ToolDyn>>,
    ) -> BoxFuture<'a, Result<String, AgentError>>;
}

pub type DynAgentProvider = Arc<dyn AgentProvider>;
