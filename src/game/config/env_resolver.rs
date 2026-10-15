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

/// Like `resolve_env`, but an unset environment variable quietly resolves to `None`
/// instead of failing — for config fields where a missing env var should disable an
/// optional feature rather than abort loading.
pub fn resolve_env_optional(value: &str) -> Option<String> {
    match value.strip_prefix('$') {
        Some(var_name) => env::var(var_name).ok(),
        None => Some(value.to_string()),
    }
}

pub fn deserialize_env_string_allow_unset<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<String>, D::Error> {
    let s = String::deserialize(d)?;
    Ok(resolve_env_optional(&s))
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

pub fn deserialize_env_u64<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrU64 {
        String(String),
        U64(u64),
    }

    match StringOrU64::deserialize(d)? {
        StringOrU64::U64(n) => Ok(n),
        StringOrU64::String(s) => {
            let resolved = resolve_env(&s).map_err(serde::de::Error::custom)?;
            resolved.parse::<u64>().map_err(serde::de::Error::custom)
        }
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
        // SAFETY: env::set_var is unsafe because it is not thread-safe; acceptable in these single-threaded test contexts.
        unsafe { env::set_var("MUDROOM_TEST_RESOLVE_ENV", "expanded") };
        assert_eq!(
            resolve_env("$MUDROOM_TEST_RESOLVE_ENV").unwrap(),
            "expanded"
        );
        unsafe { env::remove_var("MUDROOM_TEST_RESOLVE_ENV") };
    }

    #[test]
    fn resolve_env_errors_for_unset_variable() {
        assert!(resolve_env("$MUDROOM_TEST_DEFINITELY_NOT_SET_XYZ").is_err());
    }

    #[test]
    fn resolve_env_optional_returns_literal_unchanged() {
        assert_eq!(
            resolve_env_optional("http://localhost:11434"),
            Some("http://localhost:11434".to_string())
        );
    }

    #[test]
    fn resolve_env_optional_resolves_set_variable() {
        // SAFETY: env::set_var is unsafe because it is not thread-safe; acceptable in these single-threaded test contexts.
        unsafe { env::set_var("MUDROOM_TEST_RESOLVE_ENV_OPTIONAL", "expanded") };
        assert_eq!(
            resolve_env_optional("$MUDROOM_TEST_RESOLVE_ENV_OPTIONAL"),
            Some("expanded".to_string())
        );
        unsafe { env::remove_var("MUDROOM_TEST_RESOLVE_ENV_OPTIONAL") };
    }

    #[test]
    fn resolve_env_optional_returns_none_for_unset_variable() {
        assert_eq!(
            resolve_env_optional("$MUDROOM_TEST_DEFINITELY_NOT_SET_OPTIONAL_XYZ"),
            None
        );
    }

    #[test]
    fn deserialize_env_string_allow_unset_literal() {
        let toml = "value = \"http://localhost:11434\"\n";
        #[derive(serde::Deserialize)]
        struct T {
            #[serde(deserialize_with = "deserialize_env_string_allow_unset")]
            value: Option<String>,
        }
        let t: T = toml::from_str(toml).unwrap();
        assert_eq!(t.value, Some("http://localhost:11434".to_string()));
    }

    #[test]
    fn deserialize_env_string_allow_unset_resolves_set_variable() {
        // SAFETY: env::set_var is unsafe because it is not thread-safe; acceptable in these single-threaded test contexts.
        unsafe { env::set_var("MUDROOM_TEST_ALLOW_UNSET_SET", "resolved") };
        let toml = "value = \"$MUDROOM_TEST_ALLOW_UNSET_SET\"\n";
        #[derive(serde::Deserialize)]
        struct T {
            #[serde(deserialize_with = "deserialize_env_string_allow_unset")]
            value: Option<String>,
        }
        let t: T = toml::from_str(toml).unwrap();
        assert_eq!(t.value, Some("resolved".to_string()));
        unsafe { env::remove_var("MUDROOM_TEST_ALLOW_UNSET_SET") };
    }

    #[test]
    fn deserialize_env_string_allow_unset_unset_variable_is_none_not_error() {
        let toml = "value = \"$MUDROOM_TEST_ALLOW_UNSET_DEFINITELY_NOT_SET\"\n";
        #[derive(serde::Deserialize)]
        struct T {
            #[serde(deserialize_with = "deserialize_env_string_allow_unset")]
            value: Option<String>,
        }
        let t: T = toml::from_str(toml).unwrap();
        assert_eq!(t.value, None);
    }

    #[test]
    fn deserialize_env_u64_literal_integer() {
        let toml = "value = 1000\n";
        #[derive(serde::Deserialize)]
        struct T {
            #[serde(deserialize_with = "deserialize_env_u64")]
            value: u64,
        }
        let t: T = toml::from_str(toml).unwrap();
        assert_eq!(t.value, 1000);
    }

    #[test]
    fn deserialize_env_u64_env_var_string() {
        // SAFETY: env::set_var is unsafe because it is not thread-safe; acceptable in these single-threaded test contexts.
        unsafe { env::set_var("MUDROOM_TEST_U64_VAL", "5000") };
        let toml = "value = \"$MUDROOM_TEST_U64_VAL\"\n";
        #[derive(serde::Deserialize)]
        struct T {
            #[serde(deserialize_with = "deserialize_env_u64")]
            value: u64,
        }
        let t: T = toml::from_str(toml).unwrap();
        assert_eq!(t.value, 5000);
        unsafe { env::remove_var("MUDROOM_TEST_U64_VAL") };
    }

    #[test]
    fn deserialize_env_u64_unset_var_errors() {
        let toml = "value = \"$MUDROOM_TEST_DEFINITELY_NOT_SET_U64\"\n";
        #[derive(serde::Deserialize)]
        #[allow(dead_code)]
        struct T {
            #[serde(deserialize_with = "deserialize_env_u64")]
            value: u64,
        }
        assert!(toml::from_str::<T>(toml).is_err());
    }
}
