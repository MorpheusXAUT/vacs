use vacs_protocol::http::webrtc::{IceConfig, IceServer};
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;

pub(crate) const WEBRTC_TRACK_ID: &str = "audio";
pub(crate) const WEBRTC_TRACK_STREAM_ID: &str = "main";
pub(crate) const WEBRTC_CHANNELS: u16 = 1;
pub(crate) const PEER_EVENTS_CAPACITY: usize = 128;

pub trait IntoRtc<T> {
    fn into_rtc(self) -> T;
}

impl IntoRtc<RTCIceServer> for IceServer {
    fn into_rtc(self) -> RTCIceServer {
        RTCIceServer {
            urls: self.urls,
            username: self.username.unwrap_or_default(),
            credential: self.credential.unwrap_or_default(),
        }
    }
}

impl IntoRtc<RTCConfiguration> for IceConfig {
    fn into_rtc(self) -> RTCConfiguration {
        RTCConfiguration {
            ice_servers: self.ice_servers.into_iter().map(|s| s.into_rtc()).collect(),
            ..Default::default()
        }
    }
}
