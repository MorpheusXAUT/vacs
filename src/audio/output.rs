use crate::audio::{Device, EncodedAudioFrame, FRAME_SIZE, SAMPLE_RATE};
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use ringbuf::consumer::Consumer;
use ringbuf::producer::Producer;
use ringbuf::traits::Split;
use tokio::sync::mpsc::Receiver;

pub fn start_playback(
    device: &Device,
    mut rx: Receiver<EncodedAudioFrame>,
) -> Result<cpal::Stream> {
    log::debug!("Starting playback on device: {}", device);

    // We buffer 10 frames, which equals a total buffer of 200 ms at 48_000 Hz and 20 ms intervals
    let output_buffer = ringbuf::HeapRb::<f32>::new(FRAME_SIZE * 10);
    let (mut prod, mut cons) = output_buffer.split();

    let mut decoder = opus::Decoder::new(SAMPLE_RATE, opus::Channels::Mono)
        .context("Failed to create Opus decoder")?;

    tokio::spawn(async move {
        while let Some(frame) = rx.recv().await {
            let mut decoded = vec![0f32; FRAME_SIZE];
            match decoder.decode_float(&frame, &mut decoded, false) {
                Ok(decoded_samples) => {
                    prod.push_slice(&decoded[..decoded_samples]);
                }
                Err(err) => log::error!("Failed to decode output audio frame: {}", err),
            }
        }
    });

    let channels = device.stream_config.channels() as usize;

    let stream = device
        .device
        .build_output_stream(
            &device.stream_config.config(),
            move |output: &mut [f32], _| {
                let zipped = output.chunks_mut(channels).zip(cons.pop_iter());
                let zipped_len = zipped.len();
                for (chunk, sample) in zipped {
                    for c in chunk.iter_mut() {
                        *c = sample;
                    }
                }
                for out_sample in output.iter_mut().skip(zipped_len) {
                    *out_sample = cpal::Sample::EQUILIBRIUM;
                }
            },
            |err| {
                log::error!("CPAL input stream error: {}", err);
            },
            None,
        )
        .context("Failed to build input stream")?;

    stream.play().context("Failed to play output stream")?;

    log::info!("CPAL output stream started");
    Ok(stream)
}
