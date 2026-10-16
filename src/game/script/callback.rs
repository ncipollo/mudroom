use boa_engine::gc::{Finalize, Trace, empty_trace};
use boa_engine::{
    Context, JsArgs, JsError, JsNativeError, JsResult, JsValue, NativeFunction, js_string,
};
use serde::de::DeserializeOwned;
use std::cell::RefCell;
use std::rc::Rc;

use super::context::{ContextError, from_js_value};

/// Queue of callback requests recorded by a native function registered via [`register`], drained
/// by the caller after `eval()` returns. Boa's sync `eval` can't `.await`, so a callback can't
/// perform an async side effect (e.g. a DB write) itself — it records a request here instead,
/// and the Rust caller performs the real side effect afterward, back in async context.
#[derive(Debug)]
pub struct CallbackQueue<T>(Rc<RefCell<Vec<T>>>);

impl<T> Default for CallbackQueue<T> {
    fn default() -> Self {
        Self(Rc::default())
    }
}

impl<T> Clone for CallbackQueue<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<T> CallbackQueue<T> {
    /// Takes all queued requests, leaving the queue empty — a true drain, not a peek.
    pub fn drain(&self) -> Vec<T> {
        std::mem::take(&mut self.0.borrow_mut())
    }
}

impl<T> Finalize for CallbackQueue<T> {}

// SAFETY: `T` is always plain deserialized data produced by `from_js_value` (see `push_request`
// below), never a `JsValue`/`Gc` pointer, so there is nothing here for the GC to trace.
unsafe impl<T> Trace for CallbackQueue<T> {
    empty_trace!();
}

/// Registers a global JS function named `name` that, when called from a script with a single
/// JSON-shaped argument, deserializes that argument into `T` and pushes it onto the returned
/// queue. Arity is 1. This is a bare global (not namespaced under an object) — nesting under a
/// host object like `room.spawnEntity` is left to whichever caller wires up a specific callback.
pub fn register<T>(context: &mut Context, name: &str) -> Result<CallbackQueue<T>, ContextError>
where
    T: DeserializeOwned + 'static,
{
    let queue = CallbackQueue::default();
    let native_fn =
        NativeFunction::from_copy_closure_with_captures(push_request::<T>, queue.clone());
    context.register_global_builtin_callable(js_string!(name), 1, native_fn)?;
    Ok(queue)
}

fn push_request<T>(
    _this: &JsValue,
    args: &[JsValue],
    queue: &CallbackQueue<T>,
    context: &mut Context,
) -> JsResult<JsValue>
where
    T: DeserializeOwned,
{
    let arg = args.get_or_undefined(0);
    let request: T = from_js_value(arg, context).map_err(|err| {
        JsError::from(
            JsNativeError::typ().with_message(format!("invalid callback argument: {err}")),
        )
    })?;
    queue.0.borrow_mut().push(request);
    Ok(JsValue::undefined())
}

#[cfg(test)]
mod tests {
    use super::*;
    use boa_engine::Source;
    use serde::Deserialize;

    #[derive(Debug, Clone, PartialEq, Deserialize)]
    struct TestRequest {
        id: i64,
        name: String,
    }

    #[test]
    fn drain_returns_the_value_a_script_passed() {
        let mut context = Context::default();
        let queue = register::<TestRequest>(&mut context, "spawn").unwrap();
        context
            .eval(Source::from_bytes(r#"spawn({ id: 1, name: "orc" });"#))
            .unwrap();
        assert_eq!(
            queue.drain(),
            vec![TestRequest {
                id: 1,
                name: "orc".to_string()
            }]
        );
    }

    #[test]
    fn drain_preserves_call_order_across_multiple_calls() {
        let mut context = Context::default();
        let queue = register::<TestRequest>(&mut context, "spawn").unwrap();
        context
            .eval(Source::from_bytes(
                r#"spawn({ id: 1, name: "a" }); spawn({ id: 2, name: "b" });"#,
            ))
            .unwrap();
        assert_eq!(
            queue.drain(),
            vec![
                TestRequest {
                    id: 1,
                    name: "a".to_string()
                },
                TestRequest {
                    id: 2,
                    name: "b".to_string()
                },
            ]
        );
    }

    #[test]
    fn drain_on_an_unused_queue_is_empty() {
        let mut context = Context::default();
        let queue = register::<TestRequest>(&mut context, "spawn").unwrap();
        context.eval(Source::from_bytes("1 + 1;")).unwrap();
        assert_eq!(queue.drain(), Vec::new());
    }

    #[test]
    fn drain_empties_the_queue() {
        let mut context = Context::default();
        let queue = register::<TestRequest>(&mut context, "spawn").unwrap();
        context
            .eval(Source::from_bytes(r#"spawn({ id: 1, name: "a" });"#))
            .unwrap();
        queue.drain();
        assert_eq!(queue.drain(), Vec::new());
    }

    #[test]
    fn malformed_argument_fails_the_eval_call() {
        let mut context = Context::default();
        let _queue = register::<TestRequest>(&mut context, "spawn").unwrap();
        let result = context.eval(Source::from_bytes(r#"spawn({ id: "not-a-number" });"#));
        assert!(result.is_err());
    }
}
