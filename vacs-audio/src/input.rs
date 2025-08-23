use crate::{Device, DeviceType, EncodedAudioFrame, FRAME_SIZE, SAMPLE_RATE};
use anyhow::{Context, Result};
use bytes::Bytes;
use cpal::traits::{DeviceTrait, StreamTrait};
use parking_lot::Mutex;
use ringbuf::consumer::Consumer;
use ringbuf::producer::Producer;
use ringbuf::traits::Split;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::instrument;

const MAX_OPUS_FRAME_SIZE: usize = 1275; // max size of an Opus frame according to RFC 6716 3.2.1.

type InputVolumeOp = Box<dyn Fn(&mut f32) + Send>;

const INPUT_VOLUME_OPS_CAPACITY: usize = 16;
const INPUT_VOLUME_OPS_PER_DATA_CALLBACK: usize = 16;

pub struct AudioInput {
    _stream: cpal::Stream,
    volume_ops: Mutex<ringbuf::HeapProd<InputVolumeOp>>,
    muted: Arc<AtomicBool>,
}

impl AudioInput {
    #[instrument(level = "debug", skip(device, tx), err, fields(device = %device))]
    pub fn start(
        device: &Device,
        tx: mpsc::Sender<EncodedAudioFrame>,
        mut volume: f32,
        amp: f32,
    ) -> Result<Self> {
        tracing::debug!("Starting input capture on device");

        let mut frame_buf = [0.0f32; FRAME_SIZE];
        let mut frame_pos = 0usize;
        let mut encoded = vec![0u8; MAX_OPUS_FRAME_SIZE];

        let mut encoder =
            opus::Encoder::new(SAMPLE_RATE, opus::Channels::Mono, opus::Application::Voip)
                .context("Failed to create opus encoder")?;
        encoder.set_bitrate(opus::Bitrate::Max)?;
        encoder.set_inband_fec(true)?;
        encoder.set_vbr(false)?;

        let muted = Arc::new(AtomicBool::new(false));
        let muted_clone = muted.clone();

        let (ops_prod, mut ops_cons) =
            ringbuf::HeapRb::<InputVolumeOp>::new(INPUT_VOLUME_OPS_CAPACITY).split();

        let stream = device
            .device
            .build_input_stream(
                &device.stream_config.config(),
                move |data: &[f32], _| {
                    for _ in 0..INPUT_VOLUME_OPS_PER_DATA_CALLBACK {
                        if let Some(op) = ops_cons.try_pop() {
                            op(&mut volume);
                        } else {
                            break;
                        }
                    }

                    for &in_s in data {
                        let s = if muted.load(Ordering::SeqCst) {
                            0.0
                        } else {
                            in_s * amp * volume
                        };

                        frame_buf[frame_pos] = s;
                        frame_pos += 1;

                        if frame_pos == FRAME_SIZE {
                            match encoder.encode_float(&frame_buf, &mut encoded) {
                                Ok(len) => {
                                    let audio_frame = Bytes::copy_from_slice(&encoded[..len]);
                                    if let Err(err) = tx.try_send(audio_frame) {
                                        tracing::warn!(?err, "Failed to send input audio sample");
                                    }
                                }
                                Err(err) => {
                                    tracing::warn!(?err, "Failed to encode input audio frame")
                                }
                            }
                            frame_pos = 0;
                        }
                    }
                },
                |err| {
                    tracing::warn!(?err, "CPAL input stream error");
                },
                None,
            )
            .context("Failed to build input stream")?;

        tracing::trace!("Starting capture on input stream");
        stream.play().context("Failed to play input stream")?;

        tracing::info!("Successfully started audio capture on device");
        Ok(Self {
            _stream: stream,
            volume_ops: Mutex::new(ops_prod),
            muted: muted_clone,
        })
    }

    #[instrument(level = "debug", err)]
    pub fn start_default(
        tx: mpsc::Sender<EncodedAudioFrame>,
        volume: f32,
        amp: f32,
    ) -> Result<Self> {
        tracing::debug!("Starting audio input on default device");
        let default_device = Device::find_default(DeviceType::Input)?;
        Self::start(&default_device, tx, volume, amp)
    }

    pub fn start_level_meter(
        device: &Device,
        emit: Box<dyn Fn(InputLevel) + Send>,
        mut volume: f32,
        amp: f32,
    ) -> Result<Self> {
        tracing::debug!("Starting audio input level meter");

        let mut level_meter = InputLevelMeter::default();

        let (ops_prod, mut ops_cons) =
            ringbuf::HeapRb::<InputVolumeOp>::new(INPUT_VOLUME_OPS_CAPACITY).split();

        let stream = device
            .device
            .build_input_stream(
                &device.stream_config.config(),
                move |data: &[f32], _| {
                    for _ in 0..INPUT_VOLUME_OPS_PER_DATA_CALLBACK {
                        if let Some(op) = ops_cons.try_pop() {
                            op(&mut volume);
                        } else {
                            break;
                        }
                    }

                    for &in_s in data {
                        let s = in_s * amp * volume;

                        if let Some(level) = level_meter.push_sample(s) {
                            emit(level);
                        }
                    }
                },
                |err| {
                    tracing::warn!(?err, "CPAL input stream error");
                },
                None,
            )
            .context("Failed to build input stream")?;

        tracing::trace!("Starting capture on input stream");
        stream.play().context("Failed to play input stream")?;

        tracing::info!("Successfully started audio input level meter on device");
        Ok(Self {
            _stream: stream,
            volume_ops: Mutex::new(ops_prod),
            muted: Arc::new(AtomicBool::new(false)),
        })
    }

    #[instrument(level = "trace", skip(self))]
    pub fn set_volume(&mut self, volume: f32) {
        tracing::trace!("Setting volume for audio input");
        if self
            .volume_ops
            .lock()
            .try_push(Box::new(move |vol: &mut f32| *vol = volume))
            .is_err()
        {
            tracing::warn!("Failed to set volume for audio input");
        }
    }

    #[instrument(level = "trace", skip(self))]
    pub fn set_muted(&mut self, muted: bool) {
        tracing::trace!("Setting muted flag for audio input");
        self.muted.store(muted, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputLevel {
    pub dbfs_rms: f32,  // e.g. -23.4
    pub dbfs_peak: f32, // e.g. -1.2
    pub norm: f32,      // 0..1, for display purposes
    pub clipping: bool,
}

pub struct InputLevelMeter {
    window_samples: usize, // ~10-20ms worth of samples
    sum_sq: f64,
    peak: f32,
    count: usize,
    last_emit: Instant,
    emit_interval: Duration, // e.g. 16ms => ~60fps
    // smoothing (EMA in dB)
    ema_db: f32,
    attack: f32,  // 0..1, (higher = faster rise)
    release: f32, // 0..1, (lower = faster fall)
}

const INPUT_LEVEL_METER_WINDOW_MS: f32 = 15.0;

impl Default for InputLevelMeter {
    fn default() -> Self {
        let window_samples =
            ((SAMPLE_RATE as f32) * (INPUT_LEVEL_METER_WINDOW_MS / 1000.0)) as usize;

        Self {
            window_samples: window_samples.max(1),
            sum_sq: 0.0,
            peak: 0.0,
            count: 0,
            last_emit: Instant::now(),
            emit_interval: Duration::from_millis(16),
            ema_db: -90.0,
            attack: 0.5,
            release: 0.1,
        }
    }
}

const INPUT_LEVEL_MIN_DB: f32 = -60.0;
const INPUT_LEVEL_MAX_DB: f32 = 0.0;

impl InputLevelMeter {
    pub fn push_sample(&mut self, s: f32) -> Option<InputLevel> {
        let a = s.abs();
        self.peak = self.peak.max(a);
        self.sum_sq += (s as f64) * (s as f64);
        self.count += 1;

        if self.count >= self.window_samples && self.last_emit.elapsed() >= self.emit_interval {
            let rms = (self.sum_sq / (self.count as f64)).sqrt() as f32;
            let dbfs_rms = if rms > 0.0 { 20.0 * rms.log10() } else { -90.0 };
            let dbfs_peak = if self.peak > 0.0 {
                20.0 * self.peak.log10()
            } else {
                -90.0
            };

            let alpha = if dbfs_rms > self.ema_db {
                self.attack
            } else {
                self.release
            };
            self.ema_db = self.ema_db + alpha * (dbfs_rms - self.ema_db);

            let mut norm =
                (self.ema_db - INPUT_LEVEL_MIN_DB) / (INPUT_LEVEL_MAX_DB - INPUT_LEVEL_MIN_DB);
            norm = norm.clamp(0.0, 1.0);

            let clipping = self.peak >= 0.999;

            let out = InputLevel {
                dbfs_rms,
                dbfs_peak,
                norm,
                clipping,
            };

            self.sum_sq = 0.0;
            self.peak = 0.0;
            self.count = 0;
            self.last_emit = Instant::now();

            return Some(out);
        }
        None
    }
}
