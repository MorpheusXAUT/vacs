mod application_message;
mod auth;
pub mod calls;
mod client;
mod handler;
pub mod message;
#[cfg(test)]
mod test_util;
pub(crate) mod traits;

use crate::state::AppState;
use axum::Router;
use axum::routing::any;
pub use client::ClientSession;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/", any(handler::ws_handler))
}
