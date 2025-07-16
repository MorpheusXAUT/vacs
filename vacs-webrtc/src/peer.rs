use crate::config::{WEBRTC_TRACK_ID, WEBRTC_TRACK_STREAM_ID, WebrtcConfig};
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::mpsc;
use vacs_audio::{EncodedAudioFrame, SAMPLE_RATE};
use webrtc::api::APIBuilder;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MIME_TYPE_OPUS, MediaEngine};
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::TrackLocal;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;

pub struct Peer {
    peer_connection: RTCPeerConnection,
    track: Arc<TrackLocalStaticSample>,
    sender: Option<crate::Sender>,
    receiver: Option<crate::Receiver>,
}
pub type PeerConnectionState = RTCPeerConnectionState;

impl Peer {
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
                channels: 1,
                ..Default::default()
            },
            WEBRTC_TRACK_ID.to_owned(),
            WEBRTC_TRACK_STREAM_ID.to_owned(),
        ));

        peer_connection
            .add_track(Arc::clone(&track) as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .context("Failed to add track to peer connection")?;

        // todo: rework to work with channel to communicate with control
        peer_connection.on_peer_connection_state_change(Box::new(
            move |state: RTCPeerConnectionState| {
                tracing::trace!(?state, "Peer connection state changed");

                Box::pin(async {})
            },
        ));

        Ok(Self {
            peer_connection,
            track,
            sender: None,
            receiver: None,
        })
    }

    pub async fn start(
        &mut self,
        input_rx: mpsc::Receiver<EncodedAudioFrame>,
        output_tx: mpsc::Sender<EncodedAudioFrame>,
    ) -> Result<()> {
        self.sender = Some(crate::Sender::new(Arc::clone(&self.track), input_rx).await?);
        self.receiver = Some(crate::Receiver::new(&self.peer_connection, output_tx).await?);

        Ok(())
    }

    pub async fn create_offer(&self) -> Result<RTCSessionDescription> {
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

        self.await_all_ice_candidates().await;

        // update offer with all gathered ICE candidates
        let updated_offer = self
            .peer_connection
            .local_description()
            .await
            .context("Failed to get local description for offer")?;

        tracing::trace!("Created SDP offer");
        Ok(updated_offer)
    }

    pub async fn accept_offer(
        &self,
        offer: RTCSessionDescription,
    ) -> Result<RTCSessionDescription> {
        tracing::trace!("Creating SDP answer");

        self.peer_connection
            .set_remote_description(offer)
            .await
            .context("Failed to set offer as remote description")?;

        let answer = self.peer_connection.create_answer(None).await?;
        self.peer_connection
            .set_local_description(answer)
            .await
            .context("Failed to set answer as local description")?;

        self.await_all_ice_candidates().await;

        let answer = self
            .peer_connection
            .local_description()
            .await
            .context("Failed to get local description for answer")?;

        tracing::trace!("Created SDP answer");
        Ok(answer)
    }

    pub async fn accept_answer(&self, answer: RTCSessionDescription) -> Result<()> {
        tracing::trace!("Accepting SDP answer");

        self.peer_connection
            .set_remote_description(answer)
            .await
            .context("Failed to set answer as remote description")?;

        tracing::trace!("Accepted SDP answer");
        Ok(())
    }

    async fn await_all_ice_candidates(&self) {
        let (gather_complete_tx, mut gather_complete_rx) = mpsc::channel::<()>(1);

        self.peer_connection
            .on_ice_gathering_state_change(Box::new(move |state| {
                tracing::trace!(?state, "ICE gathering state changed");
                if state == webrtc::ice_transport::ice_gatherer_state::RTCIceGathererState::Complete
                {
                    match gather_complete_tx.try_send(()) {
                        Ok(()) => {}
                        Err(err) => {
                            tracing::warn!(?err, "Failed to send gather complete event")
                        }
                    }
                }

                Box::pin(async {})
            }));

        gather_complete_rx.recv().await;
    }
}
