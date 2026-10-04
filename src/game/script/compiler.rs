use std::path::Path;

use boa_engine::Source;
use boa_engine::ast::scope::Scope;
use boa_engine::interner::Interner;
use boa_engine::parser::Parser;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ScriptCompileError {
    #[error("failed to compile script `{path}`: {message}")]
    Syntax { path: String, message: String },
    #[error("failed to serialize compiled script `{path}`: {message}")]
    Serialize { path: String, message: String },
}

/// A serialized snapshot of a script's parsed, scope-analyzed AST — plain `Vec<u8>` with no
/// `Rc`/`Gc` pointers, so it's `Send + Sync` and safe to store on `Script`. Executing it still
/// needs a rebuilt `Interner` (Boa's isn't itself serializable, only its `Sym` indices are)
/// and bytecompiling in a short-lived `Context` — left to the script execution/context work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledScript(Vec<u8>);

impl CompiledScript {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Parses and scope-analyzes `source` as a JS script, catching syntax errors at load time
/// rather than mid-battle. `path` is used only to identify the script in error messages.
pub fn compile(path: &Path, source: &str) -> Result<CompiledScript, ScriptCompileError> {
    let boa_source = Source::from_bytes(source.as_bytes()).with_path(path);
    let mut interner = Interner::new();
    let scope = Scope::new_global();
    let ast = Parser::new(boa_source)
        .parse_script(&scope, &mut interner)
        .map_err(|err| ScriptCompileError::Syntax {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
    let bytes = serde_json::to_vec(&ast).map_err(|err| ScriptCompileError::Serialize {
        path: path.display().to_string(),
        message: err.to_string(),
    })?;
    Ok(CompiledScript(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_valid_script() {
        let result = compile(Path::new("abilities/fireball.js"), "1 + 1;");
        assert!(result.is_ok());
    }

    #[test]
    fn compile_error_includes_script_identity() {
        let result = compile(Path::new("abilities/bad.js"), "function (");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("abilities/bad.js"));
    }
}
