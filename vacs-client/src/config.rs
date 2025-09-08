use anyhow::Context;
use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;
use vacs_webrtc::config::WebrtcConfig;

/// User-Agent string used for all HTTP requests.
pub static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
pub const WS_LOGIN_TIMEOUT: Duration = Duration::from_secs(10);
pub const WS_READY_TIMEOUT: Duration = Duration::from_secs(10);
pub const AUDIO_SETTINGS_FILE_NAME: &str = "audio.toml";
pub const CLIENT_SETTINGS_FILE_NAME: &str = "client.toml";
pub const ENCODED_AUDIO_FRAME_BUFFER_SIZE: usize = 512;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub backend: BackendConfig,
    pub audio: AudioConfig,
    pub webrtc: WebrtcConfig,
    pub client: ClientConfig,
}

impl AppConfig {
    pub fn parse(config_dir: &Path) -> anyhow::Result<Self> {
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
            .add_source(
                File::with_name(
                    config_dir
                        .join("client.toml")
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
            base_url: if cfg!(debug_assertions) {
                "https://vacs-dev.gusch.jetzt"
            } else {
                "https://vacs.gusch.jetzt"
            }
            .to_string(),
            ws_url: if cfg!(debug_assertions) {
                "wss://vacs-dev.gusch.jetzt/ws"
            } else {
                "wss://vacs.gusch.jetzt/ws"
            }
            .to_string(),
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
    pub host_name: Option<String>, // Name of audio backend host, None means default host
    pub input_device_name: Option<String>, // None means default device
    pub output_device_name: Option<String>, // None means default device
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
            host_name: None,
            input_device_name: None,
            output_device_name: None,
            input_device_volume: 0.5,
            input_device_volume_amp: 4.0,
            output_device_volume: 0.5,
            output_device_volume_amp: 2.0,
            click_volume: 0.5,
            chime_volume: 0.5,
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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub always_on_top: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PersistedClientConfig {
    pub client: ClientConfig,
}

impl From<ClientConfig> for PersistedClientConfig {
    fn from(client: ClientConfig) -> Self {
        Self { client }
    }
}

pub trait Persistable {
    fn persist(&self, config_dir: &Path, file_name: &str) -> anyhow::Result<()>;
}

impl<T: Serialize> Persistable for T {
    fn persist(&self, config_dir: &Path, file_name: &str) -> anyhow::Result<()> {
        let serialized = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::create_dir_all(config_dir).context("Failed to create config directory")?;
        fs::write(config_dir.join(file_name), serialized)
            .context("Failed to write config to file")?;

        Ok(())
    }
}
