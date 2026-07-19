use crate::config::{
    IntoRtc, PEER_EVENTS_CAPACITY, WEBRTC_CHANNELS, WEBRTC_TRACK_ID, WEBRTC_TRACK_STREAM_ID,
};
use crate::error::WebrtcError;
use anyhow::Context;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{broadcast, mpsc};
use tracing::instrument;
use vacs_audio::{EncodedAudioFrame, TARGET_SAMPLE_RATE};
use vacs_protocol::http::webrtc::IceConfig;
use webrtc::api::APIBuilder;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MIME_TYPE_OPUS, MediaEngine};
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::TrackLocal;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;

pub type PeerConnectionState = RTCPeerConnectionState;

#[derive(Debug, Clone)]
pub enum PeerEvent {
    ConnectionState(PeerConnectionState),
    IceCandidate(String),
    Error(String),
}

pub struct Peer {
    peer_connection: RTCPeerConnection,
    track: Arc<TrackLocalStaticSample>,
    sender: Option<crate::Sender>,
    receiver: Option<crate::Receiver>,
    events_tx: broadcast::Sender<PeerEvent>,
}

impl Peer {
    #[instrument(level = "debug", err)]
    pub async fn new(
        config: IceConfig,
    ) -> Result<(Self, broadcast::Receiver<PeerEvent>), WebrtcError> {
        let mut media_engine = MediaEngine::default();
        media_engine
            .register_default_codecs()
            .context("Failed to register default codecs")?;

        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut media_engine)
            .context("Failed to register default interceptors")?;

        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build();

        let peer_connection = api
            .new_peer_connection(config.into_rtc())
            .await
            .context("Failed to create peer connection")?;

        let track = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_owned(),
                clock_rate: TARGET_SAMPLE_RATE,
                channels: WEBRTC_CHANNELS,
                ..Default::default()
            },
            WEBRTC_TRACK_ID.to_owned(),
            WEBRTC_TRACK_STREAM_ID.to_owned(),
        ));

        peer_connection
            .add_track(Arc::clone(&track) as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .context("Failed to add track to peer connection")?;

        let (events_tx, events_rx) = broadcast::channel(PEER_EVENTS_CAPACITY);

        {
            let events_tx = events_tx.clone();
            let dtls_transport = peer_connection.sctp().transport();
            peer_connection.on_peer_connection_state_change(Box::new(
                move |state: RTCPeerConnectionState| {
                    tracing::trace!(?state, "Peer connection state changed");
                    if let Err(err) = events_tx.send(PeerEvent::ConnectionState(state)) {
                        tracing::warn!(?err, "Failed to send peer connection state event");
                    }

                    let dtls_transport = Arc::clone(&dtls_transport);
                    Box::pin(async move {
                        if state == RTCPeerConnectionState::Connected {
                            match dtls_transport
                                .ice_transport()
                                .get_selected_candidate_pair()
                                .await
                            {
                                Some(pair) => {
                                    tracing::info!(%pair, "Selected ICE candidate pair");
                                }
                                None => tracing::warn!(
                                    "Connected but no selected ICE candidate pair available"
                                ),
                            }
                        }
                    })
                },
            ));
        }

        {
            let events_tx = events_tx.clone();
            let cgnat_warned = AtomicBool::new(false);
            peer_connection.on_ice_candidate(Box::new(
                move |candidate: Option<RTCIceCandidate>| {
                    tracing::trace!(?candidate, "ICE candidate received");
                    if let Some(candidate) = candidate {
                        if is_cgnat_address(&candidate.address)
                            && !cgnat_warned.swap(true, Ordering::Relaxed)
                        {
                            tracing::warn!(
                                address = %candidate.address,
                                "Local ICE candidate in CGNAT range (100.64.0.0/10), likely a VPN \
                                 interface (e.g. Cloudflare WARP, Tailscale). Calls may suffer \
                                 one-way audio; forcing relayed calls can help"
                            );
                        }

                        match candidate.to_json() {
                            Ok(init) => match serde_json::to_string(&init) {
                                Ok(init) => {
                                    if let Err(err) = events_tx.send(PeerEvent::IceCandidate(init))
                                    {
                                        tracing::warn!(?err, "Failed to send ICE candidate event");
                                    }
                                }
                                Err(err) => {
                                    tracing::warn!(?err, "Failed to serialize ICE candidate");
                                }
                            },
                            Err(err) => {
                                tracing::warn!(?err, "Failed to serialize ICE candidate");
                            }
                        }
                    }
                    Box::pin(async {})
                },
            ));
        }

        Ok((
            Self {
                peer_connection,
                track,
                sender: None,
                receiver: None,
                events_tx,
            },
            events_rx,
        ))
    }

    #[instrument(level = "debug", skip_all, err)]
    pub fn start(
        &mut self,
        input_rx: mpsc::Receiver<EncodedAudioFrame>,
        output_tx: mpsc::Sender<EncodedAudioFrame>,
    ) -> Result<(), WebrtcError> {
        tracing::debug!("Starting peer");
        if self.sender.is_some() {
            tracing::warn!("Peer sender already started");
            return Err(WebrtcError::CallActive);
        }

        if let Some(receiver) = self.receiver.as_ref() {
            tracing::trace!("Resuming receiver");
            receiver.resume(output_tx);
        } else {
            tracing::trace!("Starting receiver");
            self.receiver = Some(crate::Receiver::new(&self.peer_connection, output_tx));
        }

        self.sender = Some(crate::Sender::new(Arc::clone(&self.track), input_rx));

        tracing::trace!("Successfully started peer");
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub fn pause(&mut self) {
        tracing::debug!("Pausing peer");
        if let Some(sender) = self.sender.take() {
            sender.shutdown();
        }
        if let Some(receiver) = self.receiver.as_mut() {
            receiver.pause();
        }
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn stop(&mut self) -> Result<(), WebrtcError> {
        tracing::debug!("Stopping peer");
        if let Some(sender) = self.sender.take() {
            tracing::trace!("Shutting down sender");
            sender.stop().await?;
        }
        if let Some(receiver) = self.receiver.take() {
            tracing::trace!("Shutting down receiver");
            receiver.shutdown();
        }

        tracing::trace!("Successfully stopped peer");
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn close(&mut self) -> Result<(), WebrtcError> {
        tracing::debug!("Closing peer");
        self.stop().await.context("Failed to stop peer")?;

        tracing::trace!("Closing peer connection");
        self.peer_connection
            .close()
            .await
            .context("Failed to close peer connection")?;

        tracing::trace!("Successfully closed peer connection");
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<PeerEvent> {
        self.events_tx.subscribe()
    }

    #[instrument(level = "trace", skip(self), err)]
    pub async fn create_offer(&self) -> Result<String, WebrtcError> {
        tracing::trace!("Creating SDP offer");

        let offer = self
            .peer_connection
            .create_offer(None)
            .await
            .context("Failed to create offer")?;

        self.peer_connection
            .set_local_description(offer)
            .await
            .context("Failed to set offer as local description")?;

        let local_description = self
            .peer_connection
            .local_description()
            .await
            .context("Failed to get local description")?;

        let sdp = serde_json::to_string(&local_description)
            .context("Failed to serialize local description")?;

        tracing::trace!("Created SDP offer");
        Ok(sdp)
    }

    #[instrument(level = "trace", skip(self, sdp), err)]
    pub async fn accept_offer(&self, sdp: String) -> Result<String, WebrtcError> {
        tracing::trace!("Creating SDP answer");

        let offer = serde_json::from_str::<RTCSessionDescription>(&sdp)
            .context("Failed to deserialize SDP")?;
        self.peer_connection
            .set_remote_description(offer)
            .await
            .context("Failed to set offer as remote description")?;

        let answer = self
            .peer_connection
            .create_answer(None)
            .await
            .context("Failed to create answer")?;
        self.peer_connection
            .set_local_description(answer)
            .await
            .context("Failed to set answer as local description")?;

        let answer = self
            .peer_connection
            .local_description()
            .await
            .context("Failed to get local description for answer")?;

        let sdp =
            serde_json::to_string(&answer).context("Failed to serialize local description")?;

        tracing::trace!("Created SDP answer");
        Ok(sdp)
    }

    #[instrument(level = "trace", skip(self, sdp), err)]
    pub async fn accept_answer(&self, sdp: String) -> Result<(), WebrtcError> {
        tracing::trace!("Accepting SDP answer");

        let answer = serde_json::from_str::<RTCSessionDescription>(&sdp)
            .context("Failed to deserialize SDP")?;
        self.peer_connection
            .set_remote_description(answer)
            .await
            .context("Failed to set answer as remote description")?;

        tracing::trace!("Accepted SDP answer");
        Ok(())
    }

    #[instrument(level = "trace", skip(self, candidate), err)]
    pub async fn add_remote_ice_candidate(&self, candidate: String) -> Result<(), WebrtcError> {
        tracing::trace!("Adding remote ICE candidate");

        self.peer_connection
            .add_ice_candidate(
                serde_json::from_str::<RTCIceCandidateInit>(&candidate)
                    .context("Failed to deserialize candidate")?,
            )
            .await
            .context("Failed to add remote ICE candidate")?;

        tracing::trace!("Added remote ICE candidate");
        Ok(())
    }
}

/// Checks whether an address is within the CGNAT range 100.64.0.0/10 (RFC 6598), which is
/// commonly used by VPNs such as Cloudflare WARP or Tailscale for their virtual interfaces.
fn is_cgnat_address(address: &str) -> bool {
    match address.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => {
            let octets = ip.octets();
            octets[0] == 100 && (64..128).contains(&octets[1])
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_matches;
    use test_log::test;

    /// Building a peer without ICE servers keeps these tests offline: nothing
    /// is gathered until a local description is set.
    async fn test_peer() -> Peer {
        let (peer, _events) = Peer::new(IceConfig {
            ice_servers: Vec::new(),
            expires_at: None,
        })
        .await
        .expect("failed to create peer");
        peer
    }

    fn test_channels() -> (
        mpsc::Sender<EncodedAudioFrame>,
        mpsc::Receiver<EncodedAudioFrame>,
        mpsc::Sender<EncodedAudioFrame>,
        mpsc::Receiver<EncodedAudioFrame>,
    ) {
        let (input_tx, input_rx) = mpsc::channel(1);
        let (output_tx, output_rx) = mpsc::channel(1);
        (input_tx, input_rx, output_tx, output_rx)
    }

    #[test(tokio::test)]
    async fn start_twice_reports_call_active() {
        let mut peer = test_peer().await;

        let (_input_tx, input_rx, output_tx, _output_rx) = test_channels();
        peer.start(input_rx, output_tx)
            .expect("failed to start peer");

        let (_input_tx, input_rx, output_tx, _output_rx) = test_channels();
        assert_matches!(
            peer.start(input_rx, output_tx),
            Err(WebrtcError::CallActive)
        );
    }

    /// Holding a call stops sending but must keep the receiver alive, so
    /// incoming RTP is still drained instead of backing up behind a dead task.
    #[test(tokio::test)]
    async fn pause_stops_sending_but_keeps_receiving() {
        let mut peer = test_peer().await;

        let (_input_tx, input_rx, output_tx, _output_rx) = test_channels();
        peer.start(input_rx, output_tx)
            .expect("failed to start peer");

        peer.pause();

        assert!(peer.sender.is_none(), "pause must stop the sender");
        assert!(peer.receiver.is_some(), "pause must keep the receiver");
    }

    #[test(tokio::test)]
    async fn start_after_pause_resumes_the_call() {
        let mut peer = test_peer().await;

        let (_input_tx, input_rx, output_tx, _output_rx) = test_channels();
        peer.start(input_rx, output_tx)
            .expect("failed to start peer");
        peer.pause();

        let (_input_tx, input_rx, output_tx, _output_rx) = test_channels();
        peer.start(input_rx, output_tx)
            .expect("failed to resume peer after pause");

        assert!(peer.sender.is_some(), "resume must restore the sender");
    }

    #[test(tokio::test)]
    async fn stop_clears_sender_and_receiver() {
        let mut peer = test_peer().await;

        let (_input_tx, input_rx, output_tx, _output_rx) = test_channels();
        peer.start(input_rx, output_tx)
            .expect("failed to start peer");

        peer.stop().await.expect("failed to stop peer");

        assert!(peer.sender.is_none(), "stop must drop the sender");
        assert!(peer.receiver.is_none(), "stop must drop the receiver");
    }

    #[test]
    fn detects_cgnat_addresses() {
        assert!(is_cgnat_address("100.64.0.1"));
        assert!(is_cgnat_address("100.96.12.34"));
        assert!(is_cgnat_address("100.127.255.255"));
        assert!(!is_cgnat_address("100.63.255.255"));
        assert!(!is_cgnat_address("100.128.0.0"));
        assert!(!is_cgnat_address("192.168.1.1"));
        assert!(!is_cgnat_address("not-an-ip"));
        assert!(!is_cgnat_address("2001:db8::1"));
    }
}
