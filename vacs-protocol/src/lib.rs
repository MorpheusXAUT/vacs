#[cfg(any(feature = "http", feature = "http-webrtc"))]
pub mod http;
pub mod profile;
#[cfg(feature = "vatsim")]
pub mod vatsim;
#[cfg(feature = "ws")]
pub mod ws;

pub const VACS_PROTOCOL_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) mod sealed {
    pub trait Sealed {}
}
