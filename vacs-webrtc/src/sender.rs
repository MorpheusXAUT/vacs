use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{instrument, Instrument};
use vacs_audio::{EncodedAudioFrame, FRAME_DURATION_MS};
use webrtc::media::Sample;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;

pub struct Sender {
    shutdown_tx: watch::Sender<()>,
    task: JoinHandle<()>,
}

impl Sender {
    #[instrument(level = "trace", skip_all, err)]
    pub async fn new(
        track: Arc<TrackLocalStaticSample>,
        mut input_rx: mpsc::Receiver<EncodedAudioFrame>,
    ) -> Result<Self> {
        let (shutdown_tx, mut shutdown_rx) = watch::channel(());

        let task = tokio::runtime::Handle::current().spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    _ = shutdown_rx.changed() => {
                        tracing::trace!("Shutdown signalled, stopping sending");
                        break;
                    }
                    frame = input_rx.recv() => {
                        match frame {
                            Some(frame) => {
                                let sample = Sample {
                                    data: frame,
                                    duration: std::time::Duration::from_millis(FRAME_DURATION_MS),
                                    ..Default::default()
                                };

                                if let Err(err) = track.write_sample(&sample).await {
                                    tracing::warn!(?err, "Failed to write sample to track");
                                }
                            }
                            None => {
                                break;
                            }
                        }
                    }
                }
            }
        }.instrument(tracing::Span::current()));

        Ok(Self { shutdown_tx, task })
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    #[instrument(level = "trace", skip(self), err)]
    pub async fn stop(self) -> Result<()> {
        self.shutdown();
        tracing::trace!("Waiting for sender task to finish");
        self.task.await.context("Failed to join sender task")
    }
}
