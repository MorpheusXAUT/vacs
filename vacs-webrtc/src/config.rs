use serde::Deserialize;

pub(crate) const WEBRTC_TRACK_ID: &str = "audio";
pub(crate) const WEBRTC_TRACK_STREAM_ID: &str = "main";
pub(crate) const WEBRTC_CHANNELS: u16 = 1;
pub(crate) const PEER_EVENTS_CAPACITY: usize = 128;

#[derive(Debug, Deserialize)]
pub struct WebrtcConfig {
    pub ice_servers: Vec<String>,
}
