mod cli;
mod config;
mod audio;
mod webrtc;
mod api;

use anyhow::{Context, Result, anyhow};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bytes::Bytes;
use cpal::StreamConfig;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::fs::File;
use std::io;
use std::io::{BufWriter, Write};
use std::sync::{Arc, Mutex};
use webrtc::api::APIBuilder;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MIME_TYPE_OPUS, MediaEngine};
use webrtc::ice_transport::ice_gatherer_state::RTCIceGathererState::Complete;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::media::Sample;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::TrackLocal;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;

pub struct CircularBuffer {
    buffer: Vec<f32>,
    capacity: usize,
    read_index: usize,
    write_index: usize,
    full: bool,
    stereo_output: bool,
}

impl CircularBuffer {
    pub fn new(capacity: usize, stereo_output: bool) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            capacity,
            read_index: 0,
            write_index: 0,
            full: false,
            stereo_output,
        }
    }

    pub fn push_samples(&mut self, samples: &[f32]) {
        for &sample in samples {
            self.buffer[self.write_index] = sample;
            self.write_index = (self.write_index + 1) % self.capacity;
            if self.full {
                // Overwrite oldest sample
                self.read_index = (self.read_index + 1) % self.capacity;
            }
            self.full = self.write_index == self.read_index;
        }
    }

    pub fn pop_samples(&mut self, out: &mut [f32]) {
        let mut pop_sample = || {
            if self.read_index == self.write_index && !self.full {
                0.0
            } else {
                let sample = self.buffer[self.read_index];
                self.read_index = (self.read_index + 1) % self.capacity;
                self.full = false;
                sample
            }
        };

        if self.stereo_output {
            assert!(out.len() % 2 == 0, "Stereo output must have even length");
            for chunk in out.chunks_exact_mut(2) {
                let sample = pop_sample();
                chunk[0] = sample;
                chunk[1] = sample;
            }
        } else {
            for sample in out.iter_mut() {
                *sample = pop_sample();
            }
        }

    }


    pub fn len(&self) -> usize {
        if self.full {
            self.capacity
        } else if self.write_index >= self.read_index {
            self.write_index - self.read_index
        } else {
            self.capacity - self.read_index + self.write_index
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut m = MediaEngine::default();
    m.register_default_codecs()?;

    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut m)?;

    // cpal
    let host = cpal::default_host();
    let input_device = host
        .default_input_device()
        .context("Failed to get input device")?;

    let supported_input_config = input_device
        .supported_input_configs()?
        .filter(|c| c.sample_format() == cpal::SampleFormat::F32) // or whatever you support
        .find(|c| c.min_sample_rate().0 <= 48000 && c.max_sample_rate().0 >= 48000)
        .ok_or_else(|| anyhow!("No supported input config with 48000 Hz"))?;

    let input_device_config: StreamConfig = supported_input_config
        .with_sample_rate(cpal::SampleRate(48000))
        .into();

    let output_device = Arc::new(
        host.default_output_device()
            .context("Failed to get output device")?,
    );
    let supported_output_config = output_device
        .supported_output_configs()?
        .filter(|c| c.sample_format() == cpal::SampleFormat::F32) // or whatever you support
        .find(|c| c.min_sample_rate().0 <= 48000 && c.max_sample_rate().0 >= 48000)
        .ok_or_else(|| anyhow!("No supported output config with 48000 Hz"))?;

    let output_device_config: Arc<StreamConfig> = Arc::new(
        supported_output_config
            .with_sample_rate(cpal::SampleRate(48000))
            .into(),
    );

    // opus
    let mut encoder = opus::Encoder::new(
        input_device_config.sample_rate.0,
        opus::Channels::Mono,
        opus::Application::Audio,
    )
    .context("Failed to create opus encoder")?;
    encoder.set_bitrate(opus::Bitrate::Max)?;
    encoder.set_inband_fec(true)?;
    encoder.set_vbr(false)?;

    let decoder = Arc::new(Mutex::new(
        opus::Decoder::new(48000, opus::Channels::Mono).context("Failed to create opus encoder")?,
    ));

    println!("Sample rates:");
    println!("Input device: {}", input_device_config.sample_rate.0);
    println!("Output device: {}", output_device_config.sample_rate.0);
    println!("Opus encoder: {}", encoder.get_sample_rate()?);
    println!(
        "Opus decoder: {}",
        decoder.lock().unwrap().get_sample_rate()?
    );

    println!("Channels:");
    println!("Input device: {}", input_device_config.channels);
    println!("Output device: {}", output_device_config.channels);
    println!(
        "Output device buffer size: {:?}",
        output_device_config.buffer_size
    );

    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };

    let peer_connection = Arc::new(api.new_peer_connection(config).await?);
    let audio_track = Arc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability {
            mime_type: MIME_TYPE_OPUS.to_owned(),
            clock_rate: 48000,
            channels: 1,
            ..Default::default()
        },
        "audio".to_owned(),
        "webrtc-rs".to_owned(),
    ));

    let rtp_sender = peer_connection
        .add_track(Arc::clone(&audio_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await?;

    tokio::spawn(async move {
        let mut rtcp_buf = vec![0u8; 1500];
        while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
        println!("rtp_sender.read loop exit");
        Result::<()>::Ok(())
    });

    let (input_tx, mut input_rx) = tokio::sync::mpsc::channel::<Sample>(100);

    tokio::spawn(async move {
        while let Some(sample) = input_rx.recv().await {
            let _ = audio_track.write_sample(&sample).await;
        }
    });

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let writer = Arc::new(Mutex::new(hound::WavWriter::create(
        "mic_recording.wav",
        spec,
    )?));

    // Raw Opus writer with 2-byte BE length prefix
    let raw_file = File::create("mic_encoded.opus")?;
    let raw_writer = Arc::new(Mutex::new(BufWriter::new(raw_file)));
    let raw_clone = raw_writer.clone();

    // Input buffer to collect samples until we have 960 (20ms at 48kHz)
    const FRAME_SIZE: usize = 960;
    let input_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let input_buffer_clone = input_buffer.clone();

    let input_stream = input_device
        .build_input_stream(
            &input_device_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut buffer = input_buffer_clone.lock().unwrap();
                buffer.extend_from_slice(data);

                while buffer.len() >= FRAME_SIZE {
                    let frame: Vec<f32> = buffer.drain(..FRAME_SIZE).collect();
                    let mut encoded = vec![0u8; 4000];
                    match encoder.encode_float(&frame, &mut encoded) {
                        Ok(len) => {
                            let sample = Sample {
                                data: Bytes::copy_from_slice(&encoded[..len]),
                                duration: std::time::Duration::from_millis(20),
                                ..Default::default()
                            };
                            if let Err(e) = input_tx.try_send(sample) {
                                eprintln!("Failed to send sample to async task: {:?}", e);
                            }

                            // Write raw Opus frame: length prefix + data
                            let mut w = raw_clone.lock().unwrap();
                            let len_be = (len as u16).to_be_bytes();
                            let _ = w.write_all(&len_be);
                            let _ = w.write_all(&encoded[..len]);
                        }
                        Err(e) => {
                            eprintln!("Failed to encode packet: {:?}", e);
                        }
                    }

                    let mut writer = writer.lock().unwrap();
                    for &sample in data {
                        let s = (sample * i16::MAX as f32) as i16;
                        writer.write_sample(s).unwrap();
                    }
                }
            },
            print_stream_error,
            None,
        )
        .context("Failed to build input stream")?;
    input_stream.play().context("Failed to play input stream")?;

    peer_connection.on_track(Box::new(move |track, _, _| {
        let jitter_buffer = Arc::new(Mutex::new(CircularBuffer::new(960 * 10, output_device_config.channels == 2))); // 200ms buffer at 20ms frames
        let decoder = Arc::clone(&decoder);
        let jitter_buffer_clone = Arc::clone(&jitter_buffer);
        let output_device = Arc::clone(&output_device);
        let output_device_config = Arc::clone(&output_device_config);

        // Decoding task
        tokio::spawn(async move {
            println!("Track started");
            while let Ok((rtp, _)) = track.read_rtp().await {
                let mut decoded = vec![0f32; FRAME_SIZE];
                let start_time = std::time::Instant::now();

                match decoder
                    .lock()
                    .unwrap()
                    .decode_float(&rtp.payload, &mut decoded, false)
                {
                    Ok(decoded_samples) => {
                        let mut buffer = jitter_buffer.lock().unwrap();
                        println!(
                            "Producer: Decoded frame of {} samples in {:?}, JitterBuffer size: {}",
                            decoded_samples,
                            start_time.elapsed(),
                            buffer.len()
                        );

                        buffer.push_samples(&decoded[..decoded_samples]);
                    }
                    Err(e) => {
                        eprintln!("Opus decode error: {:?}", e);
                    }
                }
            }
        });

        // Playback task
        std::thread::spawn(move || {
            let output_stream = output_device
                .build_output_stream(
                    &output_device_config,
                    move |data: &mut [f32], _| {
                        println!("Output callback buffer length: {}", data.len());
                        let mut buffer = jitter_buffer_clone.lock().unwrap();
                        buffer.pop_samples(data);
                    },
                    print_stream_error,
                    None,
                )
                .expect("Failed to create output stream");

            output_stream.play().expect("Failed to play output stream");
            std::thread::park();
        });

        Box::pin(async {})
    }));

    let (done_tx, mut done_rx) = tokio::sync::mpsc::channel::<()>(1);

    peer_connection.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
        println!("Peer connection state changed: {}", s);

        if s == RTCPeerConnectionState::Failed {
            println!("Peer connection failed, exiting");
            let _ = done_tx.try_send(());
        }

        Box::pin(async {})
    }));

    let (gather_complete_tx, mut gather_complete_rx) = tokio::sync::mpsc::channel::<()>(1);

    peer_connection.on_ice_gathering_state_change(Box::new(move |state| {
        println!("Ice gathering state changed: {}", state);
        if state == Complete {
            let _ = gather_complete_tx.try_send(());
        }
        Box::pin(async {})
    }));

    print!("Create offer? (y/N): ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let create_offer = input.trim().eq_ignore_ascii_case("y");

    if create_offer {
        let offer = peer_connection.create_offer(None).await?;
        peer_connection
            .set_local_description(offer)
            .await
            .context("Failed to set local description")?;

        gather_complete_rx.recv().await;

        let local_desc = peer_connection
            .local_description()
            .await
            .expect("Failed to get local description including candidates");

        let b64 = BASE64_STANDARD.encode(local_desc.sdp);
        println!("Copy offer SDP: {}", b64);

        println!("Please input your answer sdp: ");
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        answer = answer.trim().to_owned();
        let answer_input = BASE64_STANDARD
            .decode(answer)
            .context("Failed to decode answer")?;
        let remote_answer = RTCSessionDescription::answer(String::from_utf8(answer_input)?)?;
        peer_connection
            .set_remote_description(remote_answer)
            .await
            .context("Failed to set remote description")?;
    } else {
        println!("Please input your offer SDP: ");
        let mut offer = String::new();
        io::stdin().read_line(&mut offer)?;
        offer = offer.trim().to_owned();
        let offer_input = BASE64_STANDARD
            .decode(offer)
            .context("Failed to decode offer")?;
        let remote_offer = RTCSessionDescription::offer(String::from_utf8(offer_input)?)?;
        peer_connection.set_remote_description(remote_offer).await?;

        let answer = peer_connection.create_answer(None).await?;
        peer_connection
            .set_local_description(answer)
            .await
            .context("Failed to set local description")?;

        gather_complete_rx.recv().await;

        let local_desc = peer_connection
            .local_description()
            .await
            .expect("Failed to get local description including candidates");

        let b64 = BASE64_STANDARD.encode(local_desc.sdp);
        println!("Copy answer SDP: {}", b64);
    }

    tokio::select! {
        _ = done_rx.recv() => {
            println!("done_rx.recv");
        },
        _ = tokio::signal::ctrl_c() => {
            println!("ctrl_c");
        }
    }

    peer_connection.close().await?;

    Ok(())
}

fn print_stream_error(error: cpal::StreamError) {
    eprintln!("input stream error: {:?}", error);
}
