use crate::config::{
    WebrtcConfig, PEER_EVENTS_CAPACITY, WEBRTC_CHANNELS, WEBRTC_TRACK_ID, WEBRTC_TRACK_STREAM_ID,
};
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::instrument;
use vacs_audio::{EncodedAudioFrame, SAMPLE_RATE};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_OPUS};
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;

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
    pub async fn new(config: WebrtcConfig) -> Result<Self> {
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

        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: config.ice_servers,
                ..Default::default()
            }],
            ..Default::default()
        };

        let peer_connection = api
            .new_peer_connection(config)
            .await
            .context("Failed to create peer connection")?;

        let track = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_owned(),
                clock_rate: SAMPLE_RATE,
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

        let (events_tx, _) = broadcast::channel(PEER_EVENTS_CAPACITY);

        {
            let events_tx = events_tx.clone();
            peer_connection.on_peer_connection_state_change(Box::new(
                move |state: RTCPeerConnectionState| {
                    tracing::trace!(?state, "Peer connection state changed");
                    if let Err(err) = events_tx.send(PeerEvent::ConnectionState(state)) {
                        tracing::warn!(?err, "Failed to send peer connection state event");
                    }
                    Box::pin(async {})
                },
            ));
        }

        {
            let events_tx = events_tx.clone();
            peer_connection.on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidate>| {
                tracing::trace!(?candidate, "ICE candidate received");
                if let Some(candidate) = candidate {
                    match candidate.to_json() {
                        Ok(init) => match serde_json::to_string(&init) {
                            Ok(init) => {
                                if let Err(err) = events_tx.send(PeerEvent::IceCandidate(init)) {
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
            }))
        }

        Ok(Self {
            peer_connection,
            track,
            sender: None,
            receiver: None,
            events_tx,
        })
    }

    #[instrument(level = "debug", skip_all, err)]
    pub async fn start(
        &mut self,
        input_rx: mpsc::Receiver<EncodedAudioFrame>,
        output_tx: mpsc::Sender<EncodedAudioFrame>,
    ) -> Result<()> {
        tracing::debug!("Starting peer");
        if self.sender.is_some() || self.receiver.is_some() {
            tracing::warn!("Peer already started");
            anyhow::bail!("Peer already started");
        }

        self.sender = Some(
            crate::Sender::new(Arc::clone(&self.track), input_rx)
                .await
                .context("Failed to create sender")?,
        );
        self.receiver = Some(
            crate::Receiver::new(&self.peer_connection, output_tx)
                .await
                .context("Failed to create receiver")?,
        );

        tracing::trace!("Successfully started peer");
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn stop(&mut self) -> Result<()> {
        tracing::debug!("Stopping peer");
        if let Some(sender) = self.sender.take() {
            tracing::trace!("Shutting down sender");
            sender.stop().await?;
        }
        if let Some(receiver) = self.receiver.take() {
            tracing::trace!("Shutting down receiver");
            receiver.shutdown()
        }

        tracing::trace!("Successfully stopped peer");
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn close(&mut self) -> Result<()> {
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
    pub async fn create_offer(&self) -> Result<String> {
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
    pub async fn accept_offer(&self, sdp: String) -> Result<String> {
        tracing::trace!("Creating SDP answer");

        let offer = serde_json::from_str::<RTCSessionDescription>(&sdp)
            .context("Failed to deserialize SDP")?;
        self.peer_connection
            .set_remote_description(offer)
            .await
            .context("Failed to set offer as remote description")?;

        let answer = self.peer_connection.create_answer(None).await?;
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
    pub async fn accept_answer(&self, sdp: String) -> Result<()> {
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
    pub async fn add_remote_ice_candidate(&self, candidate: String) -> Result<()> {
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
