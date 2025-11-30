#[cfg(any(feature = "http", feature = "http-webrtc"))]
pub mod http;
#[cfg(feature = "ws")]
pub mod ws;

pub const VACS_PROTOCOL_VERSION: &str = env!("CARGO_PKG_VERSION");
