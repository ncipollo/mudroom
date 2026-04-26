use std::env;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub provider: AgentProvider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentProvider {
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
            provider: AgentProvider::Ollama {
                base_url: "http://localhost:11434".to_string(),
                model: "llama3.2".to_string(),
            },
        }
    }
}

/// If `value` starts with `$`, look up the remainder as an environment variable name.
/// Otherwise return the value unchanged.
pub fn resolve_env(value: &str) -> Result<String, String> {
    if let Some(var_name) = value.strip_prefix('$') {
        env::var(var_name).map_err(|_| format!("environment variable '{var_name}' not set"))
    } else {
        Ok(value.to_string())
    }
}

fn deserialize_env_string<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let s = String::deserialize(d)?;
    resolve_env(&s).map_err(serde::de::Error::custom)
}

fn deserialize_env_option_string<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<String>, D::Error> {
    let opt = Option::<String>::deserialize(d)?;
    match opt {
        None => Ok(None),
        Some(s) => resolve_env(&s).map(Some).map_err(serde::de::Error::custom),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let config = AgentConfig::default_config();
        match config.provider {
            AgentProvider::Ollama { base_url, model } => {
                assert_eq!(base_url, "http://localhost:11434");
                assert_eq!(model, "llama3.2");
            }
            _ => panic!("expected Ollama provider"),
        }
    }

    #[test]
    fn resolve_env_returns_literal_unchanged() {
        assert_eq!(
            resolve_env("http://localhost:11434").unwrap(),
            "http://localhost:11434"
        );
    }

    #[test]
    fn resolve_env_expands_set_variable() {
        unsafe { env::set_var("MUDROOM_TEST_RESOLVE_ENV", "expanded") };
        assert_eq!(
            resolve_env("$MUDROOM_TEST_RESOLVE_ENV").unwrap(),
            "expanded"
        );
    }

    #[test]
    fn resolve_env_errors_for_unset_variable() {
        assert!(resolve_env("$MUDROOM_TEST_DEFINITELY_NOT_SET_XYZ").is_err());
    }

    fn round_trip(config: &AgentConfig) -> AgentConfig {
        let toml = toml::to_string(config).unwrap();
        toml::from_str(&toml).unwrap()
    }

    #[test]
    fn ollama_round_trip() {
        let config = AgentConfig {
            provider: AgentProvider::Ollama {
                base_url: "http://localhost:11434".to_string(),
                model: "llama3.2".to_string(),
            },
        };
        let rt = round_trip(&config);
        match rt.provider {
            AgentProvider::Ollama { base_url, model } => {
                assert_eq!(base_url, "http://localhost:11434");
                assert_eq!(model, "llama3.2");
            }
            _ => panic!("expected Ollama"),
        }
    }

    #[test]
    fn ollama_resolves_env_vars() {
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
            AgentProvider::Ollama { base_url, model } => {
                assert_eq!(base_url, "http://myhost:11434");
                assert_eq!(model, "mistral");
            }
            _ => panic!("expected Ollama"),
        }
    }

    #[test]
    fn anthropic_round_trip() {
        let config = AgentConfig {
            provider: AgentProvider::Anthropic {
                api_key: Some("sk-ant-123".to_string()),
                model: "claude-sonnet-4-6".to_string(),
            },
        };
        let rt = round_trip(&config);
        match rt.provider {
            AgentProvider::Anthropic { api_key, model } => {
                assert_eq!(api_key, Some("sk-ant-123".to_string()));
                assert_eq!(model, "claude-sonnet-4-6");
            }
            _ => panic!("expected Anthropic"),
        }
    }

    #[test]
    fn open_ai_round_trip() {
        let config = AgentConfig {
            provider: AgentProvider::OpenAi {
                api_key: None,
                base_url: Some("https://api.openai.com".to_string()),
                model: "gpt-4o".to_string(),
            },
        };
        let rt = round_trip(&config);
        match rt.provider {
            AgentProvider::OpenAi {
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
            provider: AgentProvider::Cohere {
                api_key: None,
                model: "command-r".to_string(),
            },
        };
        let rt = round_trip(&config);
        match rt.provider {
            AgentProvider::Cohere { api_key, model } => {
                assert_eq!(api_key, None);
                assert_eq!(model, "command-r");
            }
            _ => panic!("expected Cohere"),
        }
    }

    #[test]
    fn gemini_round_trip() {
        let config = AgentConfig {
            provider: AgentProvider::Gemini {
                api_key: Some("key".to_string()),
                model: "gemini-pro".to_string(),
            },
        };
        let rt = round_trip(&config);
        match rt.provider {
            AgentProvider::Gemini { api_key, model } => {
                assert_eq!(api_key, Some("key".to_string()));
                assert_eq!(model, "gemini-pro");
            }
            _ => panic!("expected Gemini"),
        }
    }

    #[test]
    fn x_ai_round_trip() {
        let config = AgentConfig {
            provider: AgentProvider::XAi {
                api_key: None,
                model: "grok-2".to_string(),
            },
        };
        let rt = round_trip(&config);
        match rt.provider {
            AgentProvider::XAi { api_key, model } => {
                assert_eq!(api_key, None);
                assert_eq!(model, "grok-2");
            }
            _ => panic!("expected XAi"),
        }
    }
}
