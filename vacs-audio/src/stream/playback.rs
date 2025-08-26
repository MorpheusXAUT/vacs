use crate::device::StreamDevice;
use crate::mixer::Mixer;
use crate::sources::{AudioSource, AudioSourceId};
use crate::DeviceType;
use anyhow::{Context, Result};
use cpal::traits::StreamTrait;
use parking_lot::Mutex;
use ringbuf::consumer::Consumer;
use ringbuf::producer::Producer;
use ringbuf::traits::Split;
use ringbuf::HeapRb;
use rubato::SincFixedIn;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{atomic, Arc};
use tracing::instrument;

type MixerOp = Box<dyn FnOnce(&mut Mixer) + Send>;

const MIXER_OPS_CAPACITY: usize = 256;
const MIXER_OPS_PER_DATA_CALLBACK: usize = 32;

pub struct PlaybackStream {
    _stream: cpal::Stream,
    mixer_ops: Mutex<ringbuf::HeapProd<MixerOp>>,
    next_audio_source_id: atomic::AtomicUsize,
    deafened: Arc<AtomicBool>,
    device: StreamDevice,
}

impl PlaybackStream {
    pub fn start(device: StreamDevice) -> Result<Self> {
        tracing::debug!("Starting input capture stream");
        debug_assert!(matches!(device.device_type, DeviceType::Output));

        let mut mixer = Mixer::default();
        let (ops_prod, mut ops_cons) = HeapRb::<MixerOp>::new(MIXER_OPS_CAPACITY).split();

        let deafened = Arc::new(AtomicBool::new(false));
        let deafened_clone = deafened.clone();

        let stream = device
            .build_output_stream(
                move |output, _| {
                    for _ in 0..MIXER_OPS_PER_DATA_CALLBACK {
                        if let Some(op) = ops_cons.try_pop() {
                            op(&mut mixer);
                        } else {
                            break;
                        }
                    }
                    mixer.mix(output);
                },
                |err| {
                    tracing::warn!(?err, "CPAL output stream error");
                },
            )
            .context("Failed to build output stream")?;

        tracing::debug!("Starting playback on output stream");
        stream.play().context("Failed to play output stream")?;

        Ok(Self {
            _stream: stream,
            mixer_ops: Mutex::new(ops_prod),
            next_audio_source_id: atomic::AtomicUsize::new(0),
            deafened: deafened_clone,
            device,
        })
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn stop(self) {
        tracing::info!("Stopping output playback stream");
        drop(self._stream);
    }

    pub fn set_deafened(&self, muted: bool) {
        self.deafened.store(muted, Ordering::Relaxed);
    }

    pub fn is_deafened(&self) -> bool {
        self.deafened.load(Ordering::Relaxed)
    }

    #[instrument(level = "trace", skip_all)]
    pub fn add_audio_source(&mut self, source: Box<dyn AudioSource>) -> AudioSourceId {
        let id = self
            .next_audio_source_id
            .fetch_add(1, atomic::Ordering::SeqCst);

        tracing::trace!(?id, "Adding audio source to mixer");
        if self
            .mixer_ops
            .lock()
            .try_push(Box::new(move |mixer: &mut Mixer| {
                mixer.add_source(id, source)
            }))
            .is_err()
        {
            tracing::warn!(?id, "Failed to add audio source to mixer");
        }

        id
    }

    #[instrument(level = "trace", skip(self))]
    pub fn remove_audio_source(&mut self, id: AudioSourceId) {
        tracing::trace!("Removing audio source from mixer");
        if self
            .mixer_ops
            .lock()
            .try_push(Box::new(move |mixer: &mut Mixer| mixer.remove_source(id)))
            .is_err()
        {
            tracing::warn!("Failed to remove audio source from mixer");
        }
    }

    #[instrument(level = "trace", skip(self))]
    pub fn start_audio_source(&mut self, id: AudioSourceId) {
        tracing::trace!("Starting audio source");
        if self
            .mixer_ops
            .lock()
            .try_push(Box::new(move |mixer: &mut Mixer| {
                mixer.start_source(id);
            }))
            .is_err()
        {
            tracing::warn!("Failed to start audio source");
        }
    }

    #[instrument(level = "trace", skip(self))]
    pub fn stop_audio_source(&mut self, id: AudioSourceId) {
        tracing::trace!("Stopping audio source");
        if self
            .mixer_ops
            .lock()
            .try_push(Box::new(move |mixer: &mut Mixer| {
                mixer.stop_source(id);
            }))
            .is_err()
        {
            tracing::warn!("Failed to stop audio source");
        }
    }

    #[instrument(level = "trace", skip(self))]
    pub fn restart_audio_source(&mut self, id: AudioSourceId) {
        tracing::trace!("Restarting audio source");
        if self
            .mixer_ops
            .lock()
            .try_push(Box::new(move |mixer: &mut Mixer| {
                mixer.restart_source(id);
            }))
            .is_err()
        {
            tracing::warn!("Failed to restart audio source");
        }
    }

    #[instrument(level = "trace", skip(self))]
    pub fn set_volume(&mut self, id: AudioSourceId, volume: f32) {
        tracing::trace!("Setting volume for audio source");
        if self
            .mixer_ops
            .lock()
            .try_push(Box::new(move |mixer: &mut Mixer| {
                mixer.set_source_volume(id, volume);
            }))
            .is_err()
        {
            tracing::warn!("Failed to set volume for audio source");
        }
    }

    pub fn resampler(&self) -> Result<Option<SincFixedIn<f32>>> {
        self.device.resampler()
    }

    pub fn channels(&self) -> u16 {
        self.device.channels()
    }
}
