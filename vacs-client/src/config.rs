use anyhow::Context;
use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Duration;

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
    pub fn parse() -> anyhow::Result<AppConfig> {
        Config::builder()
            .add_source(Config::try_from(&AppConfig::default())?)
            .add_source(
                File::with_name(
                    project_dirs()
                        .expect("Failed to get project dirs")
                        .config_local_dir()
                        .join("config.toml")
                        .to_str()
                        .expect("Failed to get local config path"),
                )
                .required(false),
            )
            .add_source(File::with_name("config.toml").required(false))
            .add_source(
                File::with_name(
                    project_dirs()
                        .expect("Failed to get project dirs")
                        .config_local_dir()
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
    pub input_device_name: String, // Empty string means default device
    pub output_device_name: String, // Empty string means default device
    pub input_device_volume: f32,
    pub output_device_volume: f32,
    pub click_volume: f32,
    pub chime_volume: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            input_device_name: String::new(),
            output_device_name: String::new(),
            input_device_volume: 0.5,
            output_device_volume: 0.5,
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

pub trait Persistable {
    fn persist(&self, file_name: &str) -> anyhow::Result<()>;
}

impl<T: Serialize> Persistable for T {
    fn persist(&self, file_name: &str) -> anyhow::Result<()> {
        let serialized = toml::to_string_pretty(self).context("Failed to serialize config")?;
        let config_dir = project_dirs().context("Failed to get project dirs")?;
        let config_dir = config_dir.config_local_dir();

        fs::create_dir_all(config_dir).context("Failed to create config directory")?;
        fs::write(config_dir.join(file_name), serialized)
            .context("Failed to write config to file")?;

        Ok(())
    }
}

pub fn project_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("app", "vacs", "vacs-client")
}
