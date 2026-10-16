use std::future::Future;

use boa_engine::builtins::promise::ResolvingFunctions;
use boa_engine::gc::{Finalize, Trace, empty_trace};
use boa_engine::job::NativeAsyncJob;
use boa_engine::object::builtins::JsPromise;
use boa_engine::{
    Context, JsArgs, JsError, JsNativeError, JsResult, JsString, JsValue, NativeFunction,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::context::{from_js_value, to_js_value};

/// Opts an arbitrary `'static` value out of GC tracing, for capturing plain Rust state (an
/// `Arc<GameState>`, a channel, ...) in [`register_async`]. Safe: the wrapped value can never
/// contain a Boa `Gc` pointer the collector needs to find.
#[derive(Clone)]
pub struct Captures<T: 'static>(pub T);

impl<T> Finalize for Captures<T> {}

// SAFETY: `T` is plain Rust state that never holds a Boa `Gc` pointer (see doc comment
// above), so there is nothing here for the GC to trace.
unsafe impl<T> Trace for Captures<T> {
    empty_trace!();
}

/// Registers `name` as a global JS function backed by `handler`: an async Rust fn receiving
/// the call's single argument (deserialized as `T`) plus `captures`. Returns a real `Promise`,
/// resolved with `handler`'s `Ok(R)`, or rejected (with `E`'s `Display` text) on `Err`.
pub fn register_async<T, C, H, Fut, R, E>(
    context: &mut Context,
    name: &str,
    captures: C,
    handler: H,
) -> JsResult<()>
where
    T: DeserializeOwned + 'static,
    C: Trace + Clone + 'static,
    R: Serialize + 'static,
    E: std::fmt::Display + 'static,
    Fut: Future<Output = Result<R, E>> + 'static,
    H: Fn(T, C) -> Fut + Copy + 'static,
{
    let native = NativeFunction::from_copy_closure_with_captures(
        move |_this, args, captures: &C, context| call(args, captures, context, handler),
        captures,
    );
    context.register_global_builtin_callable(JsString::from(name), 1, native)
}

fn call<T, C, H, Fut, R, E>(
    args: &[JsValue],
    captures: &C,
    context: &mut Context,
    handler: H,
) -> JsResult<JsValue>
where
    T: DeserializeOwned + 'static,
    C: Clone + 'static,
    R: Serialize + 'static,
    E: std::fmt::Display + 'static,
    Fut: Future<Output = Result<R, E>> + 'static,
    H: Fn(T, C) -> Fut + Copy + 'static,
{
    let arg: T = from_js_value(args.get_or_undefined(0), context).map_err(js_type_error)?;
    let captures = captures.clone();
    let (promise, resolvers) = JsPromise::new_pending(context);

    context.enqueue_job(
        NativeAsyncJob::new(async move |context| {
            let result = handler(arg, captures).await;
            settle(result, &resolvers, &mut context.borrow_mut())
        })
        .into(),
    );

    Ok(promise.into())
}

fn settle<R: Serialize, E: std::fmt::Display>(
    result: Result<R, E>,
    resolvers: &ResolvingFunctions,
    context: &mut Context,
) -> JsResult<JsValue> {
    match result {
        Ok(value) => {
            let js_value = to_js_value(&value, context).map_err(js_type_error)?;
            resolvers
                .resolve
                .call(&JsValue::undefined(), &[js_value], context)
        }
        Err(err) => {
            let message = JsValue::from(JsString::from(err.to_string()));
            resolvers
                .reject
                .call(&JsValue::undefined(), &[message], context)
        }
    }
}

fn js_type_error(err: impl std::fmt::Display) -> JsError {
    JsNativeError::typ().with_message(err.to_string()).into()
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use boa_engine::js_string;

    use super::*;
    use crate::game::script::execution;

    async fn double_after_delay(
        arg: i64,
        captures: Captures<Arc<AtomicBool>>,
    ) -> Result<i64, Infallible> {
        captures.0.store(true, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(5)).await;
        Ok(arg * 2)
    }

    #[tokio::test]
    async fn register_async_awaits_real_suspension_and_returns_value() {
        let (mut context, executor) = execution::new_async_context().unwrap();
        let started = Arc::new(AtomicBool::new(false));
        register_async(
            &mut context,
            "double",
            Captures(started.clone()),
            double_after_delay,
        )
        .unwrap();

        execution::run_async(
            &mut context,
            &executor,
            "globalThis.result = await double(21);",
        )
        .await
        .unwrap();

        assert!(started.load(Ordering::SeqCst));
        let value = context
            .global_object()
            .get(js_string!("result"), &mut context)
            .unwrap();
        assert_eq!(value.to_i32(&mut context).unwrap(), 42);
    }

    #[derive(Debug, thiserror::Error)]
    #[error("boom")]
    struct BoomError;

    async fn always_fails(_arg: (), _captures: ()) -> Result<(), BoomError> {
        Err(BoomError)
    }

    #[tokio::test]
    async fn register_async_rejection_surfaces_as_script_error() {
        let (mut context, executor) = execution::new_async_context().unwrap();
        register_async(&mut context, "boom", (), always_fails).unwrap();

        let result = execution::run_async(&mut context, &executor, "await boom(null);").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn run_async_evaluates_a_plain_script_with_no_callback_registered() {
        let (mut context, executor) = execution::new_async_context().unwrap();
        let value = execution::run_async(&mut context, &executor, "return 1 + 2;")
            .await
            .unwrap();
        assert_eq!(value.to_i32(&mut context).unwrap(), 3);
    }
}
