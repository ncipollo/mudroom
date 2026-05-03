pub mod anthropic;
pub mod cohere;
pub mod gemini;
pub mod ollama;
pub mod openai;
pub mod xai;

use std::sync::Arc;

use rig::message::Message;

use crate::game::config::agent_config::{AgentConfig, AgentProviderConfig};
use crate::game::entity_ai::{AgentMessage, AgentRole};

use super::provider::AgentProvider;

pub fn build_provider(config: &AgentConfig) -> Arc<dyn AgentProvider> {
    match &config.provider {
        AgentProviderConfig::Ollama { base_url, model } => Arc::new(ollama::OllamaProvider {
            base_url: base_url.clone(),
            model: model.clone(),
        }),
        AgentProviderConfig::Anthropic { api_key, model } => {
            Arc::new(anthropic::AnthropicProvider {
                api_key: api_key.clone(),
                model: model.clone(),
            })
        }
        AgentProviderConfig::OpenAi {
            api_key,
            base_url,
            model,
        } => Arc::new(openai::OpenAiProvider {
            api_key: api_key.clone(),
            base_url: base_url.clone(),
            model: model.clone(),
        }),
        AgentProviderConfig::Cohere { api_key, model } => Arc::new(cohere::CohereProvider {
            api_key: api_key.clone(),
            model: model.clone(),
        }),
        AgentProviderConfig::Gemini { api_key, model } => Arc::new(gemini::GeminiProvider {
            api_key: api_key.clone(),
            model: model.clone(),
        }),
        AgentProviderConfig::XAi { api_key, model } => Arc::new(xai::XaiProvider {
            api_key: api_key.clone(),
            model: model.clone(),
        }),
    }
}

pub fn history_to_rig(history: &[AgentMessage]) -> Vec<Message> {
    history
        .iter()
        .map(|m| match m.role {
            AgentRole::Player => Message::user(m.content.clone()),
            AgentRole::Agent => Message::assistant(m.content.clone()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::config::agent_config::AgentConfig;

    #[test]
    fn build_provider_ollama() {
        let config = AgentConfig {
            provider: AgentProviderConfig::Ollama {
                base_url: "http://localhost:11434".to_string(),
                model: "llama3.2".to_string(),
            },
            default_agent: "default".to_string(),
        };
        let _provider = build_provider(&config);
    }

    #[test]
    fn build_provider_anthropic() {
        let config = AgentConfig {
            provider: AgentProviderConfig::Anthropic {
                api_key: Some("test-key".to_string()),
                model: "claude-sonnet-4-6".to_string(),
            },
            default_agent: "default".to_string(),
        };
        let _provider = build_provider(&config);
    }

    #[test]
    fn build_provider_openai() {
        let config = AgentConfig {
            provider: AgentProviderConfig::OpenAi {
                api_key: Some("test-key".to_string()),
                base_url: None,
                model: "gpt-4o".to_string(),
            },
            default_agent: "default".to_string(),
        };
        let _provider = build_provider(&config);
    }

    #[test]
    fn build_provider_cohere() {
        let config = AgentConfig {
            provider: AgentProviderConfig::Cohere {
                api_key: Some("test-key".to_string()),
                model: "command-r".to_string(),
            },
            default_agent: "default".to_string(),
        };
        let _provider = build_provider(&config);
    }

    #[test]
    fn build_provider_gemini() {
        let config = AgentConfig {
            provider: AgentProviderConfig::Gemini {
                api_key: Some("test-key".to_string()),
                model: "gemini-pro".to_string(),
            },
            default_agent: "default".to_string(),
        };
        let _provider = build_provider(&config);
    }

    #[test]
    fn build_provider_xai() {
        let config = AgentConfig {
            provider: AgentProviderConfig::XAi {
                api_key: Some("test-key".to_string()),
                model: "grok-2".to_string(),
            },
            default_agent: "default".to_string(),
        };
        let _provider = build_provider(&config);
    }

    #[test]
    fn history_to_rig_maps_roles() {
        let history = vec![
            AgentMessage {
                role: AgentRole::Player,
                content: "Hello".to_string(),
            },
            AgentMessage {
                role: AgentRole::Agent,
                content: "Hi there!".to_string(),
            },
        ];
        let rig_history = history_to_rig(&history);
        assert_eq!(rig_history.len(), 2);
    }
}
