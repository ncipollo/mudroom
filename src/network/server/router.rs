use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};

use super::handlers::{
    ping_handler, server_info_handler, session_end_handler, session_start_handler, sse_handler,
};
use super::state::AppState;

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/server/info", get(server_info_handler))
        .route("/events", get(sse_handler))
        .route("/ping", post(ping_handler))
        .route("/session/start", post(session_start_handler))
        .route("/session/end", post(session_end_handler))
        .with_state(state)
}
