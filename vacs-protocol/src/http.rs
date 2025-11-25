use serde::{Deserialize, Serialize};

pub mod auth;
pub mod version;
pub mod webrtc;
pub mod ws;

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
}
