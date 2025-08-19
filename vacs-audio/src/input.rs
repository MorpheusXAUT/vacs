use crate::{Device, DeviceType, EncodedAudioFrame, FRAME_SIZE, SAMPLE_RATE};
use anyhow::{Context, Result};
use bytes::Bytes;
use cpal::traits::{DeviceTrait, StreamTrait};
use tokio::sync::mpsc;
use tracing::instrument;

const MAX_OPUS_FRAME_SIZE: usize = 1275;

pub struct AudioInput {
    _stream: cpal::Stream,
}

impl AudioInput {
    #[instrument(level = "debug", skip(device, tx), err, fields(device = %device))]
    pub fn start(device: &Device, tx: mpsc::Sender<EncodedAudioFrame>) -> Result<Self> {
        tracing::debug!("Starting input capture on device");

        let mut input_buffer = Vec::<f32>::new();

        let mut encoder =
            opus::Encoder::new(SAMPLE_RATE, opus::Channels::Mono, opus::Application::Voip)
                .context("Failed to create opus encoder")?;
        encoder.set_bitrate(opus::Bitrate::Max)?;
        encoder.set_inband_fec(true)?;
        encoder.set_vbr(false)?;

        let stream = device
            .device
            .build_input_stream(
                &device.stream_config.config(),
                move |data: &[f32], _| {
                    input_buffer.extend_from_slice(data);

                    while input_buffer.len() >= FRAME_SIZE {
                        let frame: Vec<f32> = input_buffer.drain(..FRAME_SIZE).collect();
                        let mut encoded = vec![0u8; MAX_OPUS_FRAME_SIZE];
                        match encoder.encode_float(&frame, &mut encoded) {
                            Ok(len) => {
                                let audio_frame = Bytes::copy_from_slice(&encoded[..len]);
                                if let Err(err) = tx.try_send(audio_frame) {
                                    tracing::warn!(?err, "Failed to send input audio sample");
                                }
                            }
                            Err(err) => tracing::warn!(?err, "Failed to encode input audio frame"),
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
        Ok(Self { _stream: stream })
    }

    #[instrument(level = "debug", err)]
    pub fn start_default(tx: mpsc::Sender<EncodedAudioFrame>) -> Result<Self> {
        tracing::debug!("Starting audio input on default device");
        let default_device = Device::find_default(DeviceType::Input)?;
        Self::start(&default_device, tx)
    }
}
