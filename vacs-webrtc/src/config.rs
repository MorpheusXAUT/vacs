use serde::{Deserialize, Serialize};

pub(crate) const WEBRTC_TRACK_ID: &str = "audio";
pub(crate) const WEBRTC_TRACK_STREAM_ID: &str = "main";
pub(crate) const WEBRTC_CHANNELS: u16 = 1;
pub(crate) const PEER_EVENTS_CAPACITY: usize = 128;

/// WebRTC configuration for low-level call setup.
///
/// This controls how peer-to-peer connections discover candidates and establish connectivity. At
/// minimum, a list of ICE servers must be provided. These are typically STUN servers, but TURN
/// servers may be added as well for NAT traversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebrtcConfig {
    /// List of ICE servers passed to the WebRTC stack.
    ///
    /// Each entry is a URL string for either a STUN or TURN server, following the
    /// [RFC 7064](https://www.rfc-editor.org/rfc/rfc7064) and [RFC 7065](https://www.rfc-editor.org/rfc/rfc7065)
    /// formats, respectively.
    ///
    /// At least one server must be provided.
    ///
    /// Example:
    /// - `stun:stun.l.google.com:19302`
    /// - `turn:turn.example.com:3478?transport=udp`
    pub ice_servers: Vec<String>,
}

impl Default for WebrtcConfig {
    fn default() -> Self {
        Self {
            // Standard public STUN server from Google
            ice_servers: vec!["stun:stun.l.google.com:19302".to_string()],
        }
    }
}
