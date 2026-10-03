use std::path::PathBuf;

/// A loaded script file. `source` holds the raw script text loaded from disk; it is not yet a
/// Boa-compiled artifact. `boa_engine::Script`/`CodeBlock` use GC-managed pointers and are not
/// `Send`/`Sync`, so they can't be cached here — `GameState` must stay `Send + Sync` since it's
/// shared via `Arc` across tokio tasks. Compilation happens per-invocation in a short-lived
/// `Context` (see the script compiler, issue #341).
#[derive(Debug, Clone, PartialEq)]
pub struct Script {
    pub path: PathBuf,
    pub source: Option<String>,
}

impl Script {
    pub fn new(path: PathBuf) -> Self {
        Self { path, source: None }
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
}
