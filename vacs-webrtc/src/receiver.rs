use anyhow::Result;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tracing::instrument;
use vacs_audio::EncodedAudioFrame;
use webrtc::peer_connection::RTCPeerConnection;

pub struct Receiver {
    shutdown_tx: watch::Sender<()>,
}

impl Receiver {
    #[instrument(level = "trace", skip_all, err)]
    pub async fn new(
        peer_connection: &RTCPeerConnection,
        output_tx: mpsc::Sender<EncodedAudioFrame>,
    ) -> Result<Self> {
        let (shutdown_tx, shutdown_rx) = watch::channel(());

        peer_connection.on_track(Box::new(move |track, _, _| {
            let output_tx = output_tx.clone();
            let mut shutdown_rx = shutdown_rx.clone();

            Box::pin(async move {
                loop {
                    tokio::select! {
                            biased;
                            _ = shutdown_rx.changed() => {
                                tracing::trace!("Shutdown signalled, stopping receiver");
                                break;
                            }
                            rtp = track.read_rtp() => {
                                match rtp {
                                    Ok((packet, _)) => {
                                        if output_tx.send(packet.payload).await.is_err() {
                                            tracing::warn!("Failed to send received RTP packet to output");
                                            break;
                                        }
                                    }
                                    Err(err) => {
                                        tracing::warn!(?err, "Failed to read RTP packet");
                                        break;
                                    }
                                }
                            }
                        }
                }
            })
        }));

        Ok(Self { shutdown_tx })
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        self.shutdown();
    }
}
