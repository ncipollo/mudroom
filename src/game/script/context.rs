pub mod character;
pub mod effect;

pub use character::CharacterContext;
pub use effect::EffectContext;

use boa_engine::{Context, JsError, JsValue};
use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContextError {
    #[error("failed to convert script context to JSON: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("failed to convert script context to a JS value: {0}")]
    Js(#[from] JsError),
    #[error("script context evaluated to a JS value with no JSON representation")]
    NotJson,
}

/// Converts a context type into a `JsValue` via JSON, the simplest way to hand a read (or
/// read/write) snapshot of Rust game state to a script without hand-rolling Boa object
/// property accessors for every field.
pub fn to_js_value<T: Serialize>(
    value: &T,
    context: &mut Context,
) -> Result<JsValue, ContextError> {
    let json = serde_json::to_value(value)?;
    Ok(JsValue::from_json(&json, context)?)
}

/// Reads a `JsValue` back into a context type via JSON — the inverse of [`to_js_value`], used
/// after a script mutates an injected context to recover the updated Rust value.
pub fn from_js_value<T: DeserializeOwned>(
    value: &JsValue,
    context: &mut Context,
) -> Result<T, ContextError> {
    let json = value.to_json(context)?.ok_or(ContextError::NotJson)?;
    Ok(serde_json::from_value(json)?)
}
