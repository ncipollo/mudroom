use serde::{Deserialize, Serialize};

use crate::game::config::env_resolver::{deserialize_env_option_string, deserialize_env_string};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub provider: AgentProviderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentProviderConfig {
    Ollama {
        #[serde(deserialize_with = "deserialize_env_string")]
        base_url: String,
        #[serde(deserialize_with = "deserialize_env_string")]
        model: String,
    },
    Anthropic {
        #[serde(default, deserialize_with = "deserialize_env_option_string")]
        api_key: Option<String>,
        #[serde(deserialize_with = "deserialize_env_string")]
        model: String,
    },
    OpenAi {
        #[serde(default, deserialize_with = "deserialize_env_option_string")]
        api_key: Option<String>,
        #[serde(default, deserialize_with = "deserialize_env_option_string")]
        base_url: Option<String>,
        #[serde(deserialize_with = "deserialize_env_string")]
        model: String,
    },
    Cohere {
        #[serde(default, deserialize_with = "deserialize_env_option_string")]
        api_key: Option<String>,
        #[serde(deserialize_with = "deserialize_env_string")]
        model: String,
    },
    Gemini {
        #[serde(default, deserialize_with = "deserialize_env_option_string")]
        api_key: Option<String>,
        #[serde(deserialize_with = "deserialize_env_string")]
        model: String,
    },
    XAi {
        #[serde(default, deserialize_with = "deserialize_env_option_string")]
        api_key: Option<String>,
        #[serde(deserialize_with = "deserialize_env_string")]
        model: String,
    },
}

impl AgentConfig {
    pub fn default_config() -> Self {
        Self {
            provider: AgentProviderConfig::Ollama {
                base_url: "http://localhost:11434".to_string(),
                model: "llama3.2".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn round_trip(config: &AgentConfig) -> AgentConfig {
        let toml = toml::to_string(config).unwrap();
        toml::from_str(&toml).unwrap()
    }

    #[test]
    fn default_config_has_expected_values() {
        let config = AgentConfig::default_config();
        match config.provider {
            AgentProviderConfig::Ollama { base_url, model } => {
                assert_eq!(base_url, "http://localhost:11434");
                assert_eq!(model, "llama3.2");
            }
            _ => panic!("expected Ollama provider"),
        }
    }

    #[test]
    fn ollama_round_trip() {
        let config = AgentConfig {
            provider: AgentProviderConfig::Ollama {
                base_url: "http://localhost:11434".to_string(),
                model: "llama3.2".to_string(),
            },
        };
        let rt = round_trip(&config);
        match rt.provider {
            AgentProviderConfig::Ollama { base_url, model } => {
                assert_eq!(base_url, "http://localhost:11434");
                assert_eq!(model, "llama3.2");
            }
            _ => panic!("expected Ollama"),
        }
    }

    #[test]
    fn ollama_resolves_env_vars() {
        // SAFETY: env::set_var is unsafe because it is not thread-safe; acceptable in these single-threaded test contexts.
        unsafe {
            env::set_var("MUDROOM_TEST_OLLAMA_URL", "http://myhost:11434");
            env::set_var("MUDROOM_TEST_OLLAMA_MODEL", "mistral");
        }
        let toml = r#"
[provider]
type = "ollama"
base_url = "$MUDROOM_TEST_OLLAMA_URL"
model = "$MUDROOM_TEST_OLLAMA_MODEL"
"#;
        let config: AgentConfig = toml::from_str(toml).unwrap();
        match config.provider {
            AgentProviderConfig::Ollama { base_url, model } => {
                assert_eq!(base_url, "http://myhost:11434");
                assert_eq!(model, "mistral");
            }
            _ => panic!("expected Ollama"),
        }
        unsafe {
            env::remove_var("MUDROOM_TEST_OLLAMA_URL");
            env::remove_var("MUDROOM_TEST_OLLAMA_MODEL");
        }
    }

    #[test]
    fn anthropic_round_trip() {
        let config = AgentConfig {
            provider: AgentProviderConfig::Anthropic {
                api_key: Some("sk-ant-123".to_string()),
                model: "claude-sonnet-4-6".to_string(),
            },
        };
        let rt = round_trip(&config);
        match rt.provider {
            AgentProviderConfig::Anthropic { api_key, model } => {
                assert_eq!(api_key, Some("sk-ant-123".to_string()));
                assert_eq!(model, "claude-sonnet-4-6");
            }
            _ => panic!("expected Anthropic"),
        }
    }

    #[test]
    fn open_ai_round_trip() {
        let config = AgentConfig {
            provider: AgentProviderConfig::OpenAi {
                api_key: None,
                base_url: Some("https://api.openai.com".to_string()),
                model: "gpt-4o".to_string(),
            },
        };
        let rt = round_trip(&config);
        match rt.provider {
            AgentProviderConfig::OpenAi {
                api_key,
                base_url,
                model,
            } => {
                assert_eq!(api_key, None);
                assert_eq!(base_url, Some("https://api.openai.com".to_string()));
                assert_eq!(model, "gpt-4o");
            }
            _ => panic!("expected OpenAi"),
        }
    }

    #[test]
    fn cohere_round_trip() {
        let config = AgentConfig {
            provider: AgentProviderConfig::Cohere {
                api_key: None,
                model: "command-r".to_string(),
            },
        };
        let rt = round_trip(&config);
        match rt.provider {
            AgentProviderConfig::Cohere { api_key, model } => {
                assert_eq!(api_key, None);
                assert_eq!(model, "command-r");
            }
            _ => panic!("expected Cohere"),
        }
    }

    #[test]
    fn gemini_round_trip() {
        let config = AgentConfig {
            provider: AgentProviderConfig::Gemini {
                api_key: Some("key".to_string()),
                model: "gemini-pro".to_string(),
            },
        };
        let rt = round_trip(&config);
        match rt.provider {
            AgentProviderConfig::Gemini { api_key, model } => {
                assert_eq!(api_key, Some("key".to_string()));
                assert_eq!(model, "gemini-pro");
            }
            _ => panic!("expected Gemini"),
        }
    }

    #[test]
    fn x_ai_round_trip() {
        let config = AgentConfig {
            provider: AgentProviderConfig::XAi {
                api_key: None,
                model: "grok-2".to_string(),
            },
        };
        let rt = round_trip(&config);
        match rt.provider {
            AgentProviderConfig::XAi { api_key, model } => {
                assert_eq!(api_key, None);
                assert_eq!(model, "grok-2");
            }
            _ => panic!("expected XAi"),
        }
    }
}
