pub mod application_message;
mod auth;
mod handler;
pub mod message;
#[cfg(test)]
pub mod test_util;
pub(crate) mod traits;

use crate::state::AppState;
use axum::Router;
use axum::routing::any;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/", any(handler::ws_handler))
}
