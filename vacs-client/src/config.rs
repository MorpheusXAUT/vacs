use anyhow::Context;
use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use vacs_audio::DeviceType;
use vacs_audio::config::AudioDeviceConfig;

/// User-Agent string used for all HTTP requests.
pub static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
pub const WS_LOGIN_TIMEOUT: Duration = Duration::from_secs(10);
pub const WS_READY_TIMEOUT: Duration = Duration::from_secs(10);
pub const AUDIO_SETTINGS_FILE_NAME: &str = "audio.toml";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub backend: BackendConfig,
    pub audio: AudioConfig,
}

impl AppConfig {
    pub fn parse(config_dir: PathBuf) -> anyhow::Result<AppConfig> {
        Config::builder()
            .add_source(Config::try_from(&AppConfig::default())?)
            .add_source(
                File::with_name(
                    config_dir
                        .join("config.toml")
                        .to_str()
                        .expect("Failed to get local config path"),
                )
                .required(false),
            )
            .add_source(File::with_name("config.toml").required(false))
            .add_source(
                File::with_name(
                    config_dir
                        .join("audio.toml")
                        .to_str()
                        .expect("Failed to get local config path"),
                )
                .required(false),
            )
            .add_source(File::with_name("audio.toml").required(false))
            .add_source(Environment::with_prefix("vacs_client"))
            .build()
            .context("Failed to build config")?
            .try_deserialize()
            .context("Failed to deserialize config")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub base_url: String,
    pub ws_url: String,
    pub endpoints: BackendEndpointsConfigs,
    pub timeout_ms: u64,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:3000".to_string(),
            ws_url: "ws://127.0.0.1:3000/ws".to_string(),
            endpoints: BackendEndpointsConfigs::default(),
            timeout_ms: 2000,
        }
    }
}

impl BackendConfig {
    pub fn endpoint_url(&self, endpoint: BackendEndpoint) -> String {
        let path = match endpoint {
            BackendEndpoint::InitAuth => &self.endpoints.init_auth,
            BackendEndpoint::ExchangeCode => &self.endpoints.exchange_code,
            BackendEndpoint::UserInfo => &self.endpoints.user_info,
            BackendEndpoint::Logout => &self.endpoints.logout,
            BackendEndpoint::WsToken => &self.endpoints.ws_token,
            BackendEndpoint::TerminateWsSession => &self.endpoints.terminate_ws_session,
        };
        format!("{}{}", self.base_url, path)
    }
}

pub enum BackendEndpoint {
    InitAuth,
    ExchangeCode,
    UserInfo,
    Logout,
    WsToken,
    TerminateWsSession,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendEndpointsConfigs {
    pub init_auth: String,
    pub exchange_code: String,
    pub user_info: String,
    pub logout: String,
    pub ws_token: String,
    pub terminate_ws_session: String,
}

impl Default for BackendEndpointsConfigs {
    fn default() -> Self {
        Self {
            init_auth: "/auth/vatsim".to_string(),
            exchange_code: "/auth/vatsim/callback".to_string(),
            user_info: "/auth/user".to_string(),
            logout: "/auth/logout".to_string(),
            ws_token: "/ws/token".to_string(),
            terminate_ws_session: "/ws".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub host_name: String, // Name of audio backend host, empty string means default host
    pub input_device_name: String, // Empty string means default device
    pub output_device_name: String, // Empty string means default device
    pub input_device_volume: f32,
    pub input_device_volume_amp: f32,
    pub output_device_volume: f32,
    pub output_device_volume_amp: f32,
    pub click_volume: f32,
    pub chime_volume: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            host_name: String::new(),
            input_device_name: String::new(),
            output_device_name: String::new(),
            input_device_volume: 0.5,
            input_device_volume_amp: 3.0,
            output_device_volume: 0.5,
            output_device_volume_amp: 2.0,
            click_volume: 0.5,
            chime_volume: 0.5,
        }
    }
}

impl AudioConfig {
    pub fn device_config(&self, device_type: DeviceType) -> AudioDeviceConfig {
        AudioDeviceConfig {
            host_name: if self.host_name.is_empty() {
                None
            } else {
                Some(self.host_name.to_string())
            },
            device_name: match device_type {
                DeviceType::Output => {
                    if self.output_device_name.is_empty() {
                        None
                    } else {
                        Some(self.output_device_name.to_string())
                    }
                }
                DeviceType::Input => {
                    if self.input_device_name.is_empty() {
                        None
                    } else {
                        Some(self.input_device_name.to_string())
                    }
                }
            },
            channels: match device_type {
                DeviceType::Output => 2,
                DeviceType::Input => 1,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PersistedAudioConfig {
    pub audio: AudioConfig,
}

impl From<AudioConfig> for PersistedAudioConfig {
    fn from(audio: AudioConfig) -> Self {
        Self { audio }
    }
}

pub trait Persistable {
    fn persist(&self, config_dir: PathBuf, file_name: &str) -> anyhow::Result<()>;
}

impl<T: Serialize> Persistable for T {
    fn persist(&self, config_dir: PathBuf, file_name: &str) -> anyhow::Result<()> {
        let serialized = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
        fs::write(config_dir.join(file_name), serialized)
            .context("Failed to write config to file")?;

        Ok(())
    }
}
