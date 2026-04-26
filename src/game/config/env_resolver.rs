use std::env;

use serde::{Deserialize, Deserializer};

/// If `value` starts with `$`, look up the remainder as an environment variable name.
/// Otherwise return the value unchanged.
pub fn resolve_env(value: &str) -> Result<String, String> {
    if let Some(var_name) = value.strip_prefix('$') {
        env::var(var_name).map_err(|_| format!("environment variable '{var_name}' not set"))
    } else {
        Ok(value.to_string())
    }
}

pub fn deserialize_env_string<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let s = String::deserialize(d)?;
    resolve_env(&s).map_err(serde::de::Error::custom)
}

pub fn deserialize_env_option_string<'de, D: Deserializer<'de>>(
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
}
