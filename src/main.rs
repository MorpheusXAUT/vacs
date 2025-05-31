use anyhow::{Context, Result};
use clap::Parser;
use config::{Config, Environment, File};
use log::LevelFilter;
use tokio::sync::mpsc;
use vvcs::audio;
use vvcs::config::LoggingConfig;

#[tokio::main]
async fn main() -> Result<()> {
    let _cli = parse_args();
    let config = load_config()?;
    init_logger(&config.logging);

    log::trace!("Parsed config: {:?}", config);

    let input_device = audio::Device::new(&config.audio.input, audio::DeviceType::Input)?;
    let output_device = audio::Device::new(&config.audio.output, audio::DeviceType::Output)?;

    let (input_tx, _input_rx) = mpsc::channel::<audio::EncodedAudioFrame>(32);
    let _input_stream = audio::input::start_capture(&input_device, input_tx)?;

    let (_output_tx, output_rx) = mpsc::channel::<audio::EncodedAudioFrame>(32);
    let _output_stream = audio::output::start_playback(&output_device, output_rx)?;

    Ok(())
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

fn load_config() -> Result<vvcs::config::AppConfig> {
    let settings = Config::builder()
        // Defaults
        .set_default("api.url", "http://localhost:8080")?
        .set_default("api.key", "supersikrit")?
        .set_default("webrtc.ice_servers", vec!["stun:stun.l.google.com:19302"])?
        .set_default("logging.level", LevelFilter::max().as_str())?
        .set_default("audio.input.channels", 1)?
        .set_default("audio.output.channels", 2)?
        // Config files overriding defaults
        .add_source(
            File::with_name(
                directories::ProjectDirs::from("com", "vvcs", "vvcs")
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
        .add_source(Environment::with_prefix("VVCS"));

    settings
        .build()?
        .try_deserialize()
        .context("Failed to deserialize config")
}

fn init_logger(config: &LoggingConfig) {
    env_logger::builder().filter_level(config.level).init();
}
