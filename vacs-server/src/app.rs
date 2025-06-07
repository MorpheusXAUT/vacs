use crate::state::AppState;
use crate::ws::ws_handler;
use axum::routing::any;
use axum::Router;
use std::sync::Arc;
use std::time::Duration;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

pub fn create_app() -> Router<Arc<AppState>> {
    Router::new().route("/ws", any(ws_handler)).layer((
        TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true)),
        TimeoutLayer::new(Duration::from_secs(10)),
    ))
}
