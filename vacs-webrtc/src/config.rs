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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use test_log::test;

    #[test]
    fn ice_server_keeps_turn_credentials() {
        let server = IceServer {
            urls: vec!["turn:turn.example.org:3478".to_owned()],
            username: Some("user".to_owned()),
            credential: Some("secret".to_owned()),
        };

        let rtc = server.into_rtc();

        assert_eq!(rtc.urls, vec!["turn:turn.example.org:3478".to_owned()]);
        assert_eq!(rtc.username, "user");
        assert_eq!(rtc.credential, "secret");
    }

    /// STUN-only servers carry no credentials, and the webrtc crate expects
    /// empty strings rather than an absent value to treat them as anonymous.
    #[test]
    fn ice_server_without_credentials_maps_to_empty_strings() {
        let server = IceServer {
            urls: vec!["stun:stun.example.org:3478".to_owned()],
            username: None,
            credential: None,
        };

        let rtc = server.into_rtc();

        assert_eq!(rtc.username, "");
        assert_eq!(rtc.credential, "");
    }

    #[test]
    fn ice_config_maps_every_server_in_order() {
        let config = IceConfig {
            ice_servers: vec![
                IceServer {
                    urls: vec!["stun:stun.example.org:3478".to_owned()],
                    username: None,
                    credential: None,
                },
                IceServer {
                    urls: vec![
                        "turn:turn.example.org:3478".to_owned(),
                        "turn:turn.example.org:5349?transport=tcp".to_owned(),
                    ],
                    username: Some("user".to_owned()),
                    credential: Some("secret".to_owned()),
                },
            ],
            expires_at: None,
        };

        let rtc = config.into_rtc();

        assert_eq!(rtc.ice_servers.len(), 2);
        assert_eq!(
            rtc.ice_servers[0].urls,
            vec!["stun:stun.example.org:3478".to_owned()]
        );
        assert_eq!(rtc.ice_servers[1].urls.len(), 2);
        assert_eq!(rtc.ice_servers[1].credential, "secret");
    }
}
