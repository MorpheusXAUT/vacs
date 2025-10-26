use serde::{Deserialize, Serialize};

pub(crate) const WEBRTC_TRACK_ID: &str = "audio";
pub(crate) const WEBRTC_TRACK_STREAM_ID: &str = "main";
pub(crate) const WEBRTC_CHANNELS: u16 = 1;
pub(crate) const PEER_EVENTS_CAPACITY: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebrtcConfig {
    pub ice_servers: Vec<String>,
}

impl Default for WebrtcConfig {
    fn default() -> Self {
        Self {
            ice_servers: vec!["stun:stun.l.google.com:19302".to_string()],
        }
    }
}
