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

/// webrtc-rs only implements TURN over UDP: `turns:` (TLS) and `?transport=tcp` URLs are
/// rejected during relay candidate gathering ("Unable to handle URL in gather_candidates_relay"),
/// so they are dropped here to avoid useless gathering attempts and log noise.
fn is_supported_ice_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    if lower.starts_with("turns:") {
        return false;
    }
    if lower.starts_with("turn:") {
        return !lower
            .split_once('?')
            .is_some_and(|(_, query)| query.contains("transport=tcp"));
    }
    true
}

impl IntoRtc<RTCIceServer> for IceServer {
    fn into_rtc(self) -> RTCIceServer {
        let (supported, unsupported): (Vec<String>, Vec<String>) = self
            .urls
            .into_iter()
            .partition(|url| is_supported_ice_url(url));

        if !unsupported.is_empty() {
            tracing::debug!(
                ?unsupported,
                "Skipping ICE server URLs unsupported by webrtc-rs (TURN is only supported over UDP)"
            );
        }

        RTCIceServer {
            urls: supported,
            username: self.username.unwrap_or_default(),
            credential: self.credential.unwrap_or_default(),
        }
    }
}

impl IntoRtc<RTCConfiguration> for IceConfig {
    fn into_rtc(self) -> RTCConfiguration {
        RTCConfiguration {
            ice_servers: self
                .ice_servers
                .into_iter()
                .map(|s| s.into_rtc())
                .filter(|s| !s.urls.is_empty())
                .collect(),
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
                        "turn:turn.example.org:5349?transport=udp".to_owned(),
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

    #[test]
    fn filters_unsupported_turn_urls() {
        let server = IceServer {
            urls: vec![
                "stun:stun.vacs.network:3478".to_owned(),
                "turn:turn.vacs.network:3478?transport=udp".to_owned(),
                "turn:turn.vacs.network:3478?transport=tcp".to_owned(),
                "turns:turn.vacs.network:5349?transport=tcp".to_owned(),
            ],
            username: None,
            credential: None,
        };

        let rtc = server.into_rtc();

        assert_eq!(
            rtc.urls,
            vec![
                "stun:stun.vacs.network:3478".to_owned(),
                "turn:turn.vacs.network:3478?transport=udp".to_owned(),
            ]
        );
    }

    #[test]
    fn keeps_turn_url_without_transport_param() {
        assert!(is_supported_ice_url("turn:turn.vacs.network:3478"));
    }
}
