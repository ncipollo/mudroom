use std::collections::HashMap;
use std::fs;

use thiserror::Error;
use tokio::sync::RwLock;

use super::Script;
use super::compiler::{self, ScriptCompileError};

#[derive(Debug, Error)]
pub enum ScriptCacheError {
    #[error("no script registered for `{0}`")]
    NotFound(String),
    #[error("failed to read script `{path}`: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Compile(#[from] ScriptCompileError),
}

/// Returns the source of the script registered under `name`, reading it from disk and
/// compiling it (to catch syntax errors and populate `Script.compiled`) on first use only —
/// every later lookup of the same name reuses the cached source without touching disk again.
pub async fn load_source(
    scripts: &RwLock<HashMap<String, Script>>,
    name: &str,
) -> Result<String, ScriptCacheError> {
    if let Some(source) = cached_source(scripts, name).await {
        return Ok(source);
    }
    compile_and_cache(scripts, name).await
}

async fn cached_source(scripts: &RwLock<HashMap<String, Script>>, name: &str) -> Option<String> {
    scripts.read().await.get(name)?.source.clone()
}

async fn compile_and_cache(
    scripts: &RwLock<HashMap<String, Script>>,
    name: &str,
) -> Result<String, ScriptCacheError> {
    let mut scripts = scripts.write().await;
    let script = scripts
        .get_mut(name)
        .ok_or_else(|| ScriptCacheError::NotFound(name.to_string()))?;
    if let Some(source) = &script.source {
        return Ok(source.clone());
    }
    let source = fs::read_to_string(&script.path).map_err(|err| ScriptCacheError::Io {
        path: script.path.display().to_string(),
        source: err,
    })?;
    script.compiled = Some(compiler::compile(&script.path, &source)?);
    script.source = Some(source.clone());
    Ok(source)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;

    fn registry(name: &str, script: Script) -> RwLock<HashMap<String, Script>> {
        let mut map = HashMap::new();
        map.insert(name.to_string(), script);
        RwLock::new(map)
    }

    #[tokio::test]
    async fn load_source_reads_and_compiles_on_first_use() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("damage_bonus.js");
        fs::write(&path, "1 + 1;").unwrap();
        let scripts = registry("abilities/damage_bonus", Script::new(path));

        let source = load_source(&scripts, "abilities/damage_bonus")
            .await
            .unwrap();
        assert_eq!(source, "1 + 1;");

        let cached = scripts.read().await;
        let script = &cached["abilities/damage_bonus"];
        assert!(script.source.is_some());
        assert!(script.compiled.is_some());
    }

    #[tokio::test]
    async fn load_source_reuses_cache_without_rereading_disk() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("damage_bonus.js");
        fs::write(&path, "1 + 1;").unwrap();
        let scripts = registry("abilities/damage_bonus", Script::new(path.clone()));

        load_source(&scripts, "abilities/damage_bonus")
            .await
            .unwrap();
        fs::remove_file(&path).unwrap();

        let source = load_source(&scripts, "abilities/damage_bonus")
            .await
            .unwrap();
        assert_eq!(source, "1 + 1;");
    }

    #[tokio::test]
    async fn load_source_errors_when_name_is_unregistered() {
        let scripts: RwLock<HashMap<String, Script>> = RwLock::new(HashMap::new());
        let result = load_source(&scripts, "abilities/missing").await;
        assert!(matches!(result, Err(ScriptCacheError::NotFound(_))));
    }

    #[tokio::test]
    async fn load_source_errors_when_file_is_missing() {
        let scripts = registry(
            "abilities/damage_bonus",
            Script::new(PathBuf::from("/nonexistent/damage_bonus.js")),
        );
        let result = load_source(&scripts, "abilities/damage_bonus").await;
        assert!(matches!(result, Err(ScriptCacheError::Io { .. })));
    }

    #[tokio::test]
    async fn load_source_errors_on_invalid_syntax() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.js");
        fs::write(&path, "function (").unwrap();
        let scripts = registry("abilities/bad", Script::new(path));

        let result = load_source(&scripts, "abilities/bad").await;
        assert!(matches!(result, Err(ScriptCacheError::Compile(_))));
    }
}
