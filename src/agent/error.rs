use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("tool error: {0}")]
    Tool(String),
    #[error("no provider configured")]
    NotConfigured,
}
