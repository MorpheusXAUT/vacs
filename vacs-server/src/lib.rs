pub mod auth;
pub mod build;
pub mod config;
pub mod http;
pub mod routes;
pub mod state;
pub mod store;
#[cfg(feature = "test-utils")]
pub mod test_utils;
pub mod ws;

/// User-Agent string used for all HTTP requests.
static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
