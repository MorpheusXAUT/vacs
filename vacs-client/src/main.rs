use anyhow::{Context, Result};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use clap::Parser;
use config::{Config, Environment, File};
use std::io;
use tokio::sync::{mpsc, watch};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use vacs_core::audio;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    let _cli = parse_args();
    let config = load_config()?;

    tracing::trace!(?config, "Parsed config");

    let input_device = audio::Device::new(&config.audio.input, audio::DeviceType::Input)?;
    let output_device = audio::Device::new(&config.audio.output, audio::DeviceType::Output)?;

    let mut peer = vacs_core::webrtc::Peer::new(config.webrtc)
        .await
        .context("Failed to create webrtc peer")?;

    print!("Create offer? (y/N): ");
    let create_offer = read_stdin()?.eq_ignore_ascii_case("y");

    if create_offer {
        let offer = peer.create_offer().await?;

        println!("Copy offer SDP: {}", BASE64_STANDARD.encode(offer.sdp));

        println!("Paste answer SDP: ");
        let answer_input = BASE64_STANDARD
            .decode(read_stdin()?)
            .context("Failed to decode answer")?;
        let answer = RTCSessionDescription::answer(String::from_utf8(answer_input)?)?;

        peer.accept_answer(answer).await?;
    } else {
        println!("Paste offer SDP: ");
        let offer_input = BASE64_STANDARD
            .decode(read_stdin()?)
            .context("Failed to decode offer")?;
        let offer = RTCSessionDescription::offer(String::from_utf8(offer_input)?)?;

        let answer = peer.accept_offer(offer).await?;

        println!("Copy answer SDP: {}", BASE64_STANDARD.encode(answer.sdp));
    }

    let (input_tx, input_rx) = mpsc::channel::<audio::EncodedAudioFrame>(32);
    let (output_tx, output_rx) = mpsc::channel::<audio::EncodedAudioFrame>(32);

    peer.start(input_rx, output_tx)
        .await
        .context("Failed to start peer")?;

    let _input_stream = audio::input::start_capture(&input_device, input_tx)?;

    let _output_stream = audio::output::start_playback(&output_device, output_rx)?;

    let (_done_tx, mut done_rx) = watch::channel(());

    tokio::select! {
        _ = done_rx.changed() => {
            tracing::info!("Received done signal, exiting");
        },
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received ctrl-c, exiting");
        }
    }

    Ok(())
}

fn read_stdin() -> Result<String> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

#[derive(Parser, Debug)]
#[command(version)]
#[command(about = "VATSIM Voice Communication System")]
#[command(
    long_about = "A VATSIM Voice Communication System for ground to ground communication between controllers and pilots"
)]
pub struct CliArgs {}

fn parse_args() -> CliArgs {
    CliArgs::parse()
}

fn load_config() -> Result<vacs_core::config::AppConfig> {
    let settings = Config::builder()
        // Defaults
        .set_default("api.url", "http://localhost:8080")?
        .set_default("api.key", "supersikrit")?
        .set_default("webrtc.ice_servers", vec!["stun:stun.l.google.com:19302"])?
        .set_default("audio.input.channels", 1)?
        .set_default("audio.output.channels", 2)?
        // Config files overriding defaults
        .add_source(
            File::with_name(
                directories::ProjectDirs::from("app", "vacs", "vacs-client")
                    .expect("Failed to get project dirs")
                    .config_local_dir()
                    .join("config.toml")
                    .to_str()
                    .expect("Failed to get local config path"),
            )
            .required(false),
        )
        .add_source(File::with_name("config.toml").required(false))
        // Environment variables overriding config files
        .add_source(Environment::with_prefix("vacs_client"));

    settings
        .build()?
        .try_deserialize()
        .context("Failed to deserialize config")
}

fn init_logger() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=trace,vacs_core=trace", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
