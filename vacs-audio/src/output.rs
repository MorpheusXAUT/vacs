use crate::mixer::Mixer;
use crate::sources::{AudioSource, AudioSourceId};
use crate::{Device, DeviceType};
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use ringbuf::consumer::Consumer;
use ringbuf::producer::Producer;
use ringbuf::traits::Split;
use std::sync::atomic;
use tracing::instrument;

type MixerOp = Box<dyn FnOnce(&mut Mixer) + Send>;

const MIXER_OPS_CAPACITY: usize = 256;
const MIXER_OPS_PER_DATA_CALLBACK: usize = 32;

pub struct AudioOutput {
    _stream: cpal::Stream,
    mixer_ops: ringbuf::HeapProd<MixerOp>,
    next_audio_source_id: atomic::AtomicUsize,
}

impl AudioOutput {
    #[instrument(level = "debug", skip(device), err, fields(device = %device))]
    pub fn start(device: &Device) -> Result<Self> {
        tracing::debug!("Starting audio output on device");

        let mut mixer = Mixer::default();
        let (ops_prod, mut ops_cons) = ringbuf::HeapRb::<MixerOp>::new(MIXER_OPS_CAPACITY).split();

        let stream = device
            .device
            .build_output_stream(
                &device.stream_config.config(),
                move |output: &mut [f32], _| {
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
                    tracing::warn!(?err, "CPAL input stream error");
                },
                None,
            )
            .context("Failed to build input stream")?;

        tracing::trace!("Starting playback on output stream");
        stream.play().context("Failed to play output stream")?;

        tracing::info!("Successfully started audio output on device");
        Ok(Self {
            _stream: stream,
            mixer_ops: ops_prod,
            next_audio_source_id: atomic::AtomicUsize::new(0),
        })
    }

    #[instrument(level = "debug", err)]
    pub fn start_default() -> Result<Self> {
        tracing::debug!("Starting audio output on default device");
        let default_device = Device::find_default(DeviceType::Output)?;
        Self::start(&default_device)
    }

    pub fn stop(self) {
        drop(self);
    }

    #[instrument(level = "trace", skip_all)]
    pub fn add_audio_source(&mut self, source: Box<dyn AudioSource>) -> AudioSourceId {
        let id = self
            .next_audio_source_id
            .fetch_add(1, atomic::Ordering::SeqCst);

        tracing::trace!(?id, "Adding audio source to mixer");
        if self
            .mixer_ops
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
            .try_push(Box::new(move |mixer: &mut Mixer| {
                mixer.set_source_volume(id, volume);
            }))
            .is_err()
        {
            tracing::warn!("Failed to set volume for audio source");
        }
    }
}
