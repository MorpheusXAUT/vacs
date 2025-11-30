use serde::{Deserialize, Serialize};

#[cfg(feature = "http")]
pub mod auth;
#[cfg(feature = "http")]
pub mod version;
#[cfg(feature = "http-webrtc")]
pub mod webrtc;
#[cfg(feature = "http")]
pub mod ws;

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
}
