use crate::audio::{Device, EncodedAudioFrame, FRAME_SIZE, SAMPLE_RATE};
use anyhow::{Context, Result};
use bytes::Bytes;
use cpal::traits::{DeviceTrait, StreamTrait};
use tokio::sync::mpsc::Sender;

const MAX_OPUS_FRAME_SIZE: usize = 1275;

pub fn start_capture(device: &Device, tx: Sender<EncodedAudioFrame>) -> Result<cpal::Stream> {
    log::debug!("Starting capture on device: {}", device);

    let mut encoder =
        opus::Encoder::new(SAMPLE_RATE, opus::Channels::Mono, opus::Application::Voip)
            .context("Failed to create opus encoder")?;
    encoder.set_bitrate(opus::Bitrate::Max)?;
    encoder.set_inband_fec(true)?;
    encoder.set_vbr(false)?;

    log::trace!("Opus encoder: {:?}", encoder);

    let mut input_buffer = Vec::<f32>::new();

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
                                log::error!("Failed to send input audio sample: {}", err);
                            }
                        }
                        Err(err) => log::error!("Failed to encode input audio frame: {}", err),
                    }
                }
            },
            |err| {
                log::error!("CPAL input stream error: {}", err);
            },
            None,
        )
        .context("Failed to build input stream")?;

    stream.play().context("Failed to play input stream")?;

    log::info!("CPAL input stream started");

    Ok(stream)
}
