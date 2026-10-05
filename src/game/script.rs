use std::path::PathBuf;

pub mod compiler;
pub mod context;

pub use compiler::{CompiledScript, ScriptCompileError, compile};

/// A loaded script file. `source` is the raw text loaded from disk; `compiled` is the
/// serialized snapshot produced by [`compile`]. Boa's own compiled forms are GC-managed and
/// not `Send`/`Sync` (unlike `GameState`, shared via `Arc`), so only the serialized form is
/// cached here — see [`CompiledScript`] for what rebuilding an executable form still needs.
#[derive(Debug, Clone, PartialEq)]
pub struct Script {
    pub path: PathBuf,
    pub source: Option<String>,
    pub compiled: Option<CompiledScript>,
}

impl Script {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            source: None,
            compiled: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_path() {
        let script = Script::new(PathBuf::from("abilities/fireball.js"));
        assert_eq!(script.path, PathBuf::from("abilities/fireball.js"));
    }

    #[test]
    fn new_defaults_source_to_none() {
        let script = Script::new(PathBuf::from("abilities/fireball.js"));
        assert_eq!(script.source, None);
    }

    #[test]
    fn new_defaults_compiled_to_none() {
        let script = Script::new(PathBuf::from("abilities/fireball.js"));
        assert_eq!(script.compiled, None);
    }
}
