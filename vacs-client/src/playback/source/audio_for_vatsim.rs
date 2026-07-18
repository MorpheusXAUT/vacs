use crate::playback::{
    PlaybackError, TapId,
    source::{
        PlaybackSource, PlaybackSourceEvent,
        capture::{CaptureSource, DefaultLoopbackCapture, LoopbackCapture, LoopbackEvent},
    },
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use trackaudio::Frequency;

const EVENT_CHANNEL_CAPACITY: usize = 1024;

pub struct AudioForVatsimLoopbackSource {
    capture: Option<DefaultLoopbackCapture>,
    cancel: CancellationToken,
    forwarder: Option<JoinHandle<()>>,
}

impl AudioForVatsimLoopbackSource {
    pub fn new() -> Self {
        Self {
            capture: None,
            cancel: CancellationToken::new(),
            forwarder: None,
        }
    }
}

#[async_trait::async_trait]
impl PlaybackSource for AudioForVatsimLoopbackSource {
    async fn start(&mut self) -> Result<mpsc::Receiver<PlaybackSourceEvent>, PlaybackError> {
        let (tx, rx) = mpsc::channel::<PlaybackSourceEvent>(EVENT_CHANNEL_CAPACITY);

        let (capture, mut capture_rx) =
            DefaultLoopbackCapture::start(CaptureSource::AudioForVatsim)?;

        let cancel = self.cancel.clone();

        let forwarder = tokio::spawn(async move {
            let mut receiving = false;

            loop {
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => {
                        log::trace!("forwarder cancelled");
                        break;
                    }

                    evt = capture_rx.recv() => {
                        let Some(evt) = evt else {
                            log::warn!("loopback capture channel closed");
                            break;
                        };
                        let (mapped, second) = match evt {
                            LoopbackEvent::Opened { tap, sample_rate, channels } => {
                                (PlaybackSourceEvent::TapOpened { tap, sample_rate, channels }, None)
                            }
                            LoopbackEvent::Closed { tap } => {
                                (PlaybackSourceEvent::TapClosed { tap }, None)
                            }
                            LoopbackEvent::Frame { tap, samples, captured_at } => {
                                let mut second = None;
                                if !receiving && is_starting(samples.as_ref()) {
                                    receiving = true;
                                    second = Some(PlaybackSourceEvent::RxBegin {
                                        tap: TapId::Headset,
                                        callsign: "".to_string(),
                                        frequency: Frequency::from_hz(0),
                                    });
                                } else if receiving && is_ending(samples.as_ref()) {
                                    receiving = false;
                                    second = Some(PlaybackSourceEvent::RxEnd {
                                        callsign: "".to_string(),
                                        frequency: Frequency::from_hz(0),
                                        active_transmitters: None,
                                    });
                                }

                                (PlaybackSourceEvent::Frame { tap, samples, captured_at }, second)
                            },
                        };
                        if tx.send(mapped).await.is_err() {
                            break;
                        }
                        if let Some(second) = second {
                            let _ = tx.send(second).await;
                        }
                    }
                }
            }
        });

        self.capture = Some(capture);
        self.forwarder = Some(forwarder);
        Ok(rx)
    }

    async fn stop(&mut self) {
        self.cancel.cancel();
        if let Some(forwarder) = self.forwarder.take()
            && let Err(err) = forwarder.await
        {
            log::warn!("audio-for-vatsim playback forwarder task panicked: {err}");
        }
        if let Some(mut capture) = self.capture.take() {
            capture.stop();
        }
    }
}

fn is_starting(samples: &[f32]) -> bool {
    samples.iter().skip_while(|x| **x == 0.0f32).count() > 0
}

fn is_ending(samples: &[f32]) -> bool {
    samples.iter().rev().take(3).all(|x| *x == 0.0f32)
}
