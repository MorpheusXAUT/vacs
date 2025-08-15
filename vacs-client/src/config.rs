use anyhow::Context;
use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Duration;
use anyhow::Context;
use config::{Config, Environment, File};
use serde::Deserialize;

/// User-Agent string used for all HTTP requests.
pub static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
pub const WS_LOGIN_TIMEOUT: Duration = Duration::from_secs(10);
pub const WS_READY_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub backend: BackendConfig,
    pub audio: AudioConfig,
}

impl AppConfig {
    pub fn parse() -> anyhow::Result<AppConfig> {
        Config::builder()
            .set_default("backend.base_url", "http://127.0.0.1:3000")?
            .set_default("backend.ws_url", "ws://127.0.0.1:3000/ws")?
            .set_default("backend.endpoints.init_auth", "/auth/vatsim")?
            .set_default("backend.endpoints.exchange_code", "/auth/vatsim/callback")?
            .set_default("backend.endpoints.user_info", "/auth/user")?
            .set_default("backend.endpoints.logout", "/auth/logout")?
            .set_default("backend.endpoints.ws_token", "/ws/token")?
            .set_default("backend.endpoints.terminate_ws_session", "/ws")?
            .set_default("backend.timeout_ms", 2000)?
            // .set_default("audio.input_device", "")?
            // .set_default("audio.output_device", "")?
            .add_source(Config::try_from(&PersistedAudioConfig::default())?)
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
            .add_source(File::with_name("audio.toml").required(false)) // TODO: How about this?
            .add_source(Environment::with_prefix("vacs_client"))
            .build()
            .context("Failed to build config")?
            .try_deserialize()
            .context("Failed to deserialize config")
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BackendConfig {
    pub base_url: String,
    pub ws_url: String,
    pub endpoints: BackendEndpointsConfigs,
    pub timeout_ms: u64,
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

#[derive(Debug, Clone, Deserialize)]
pub struct BackendEndpointsConfigs {
    pub init_auth: String,
    pub exchange_code: String,
    pub user_info: String,
    pub logout: String,
    pub ws_token: String,
    pub terminate_ws_session: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioConfig {
    pub input_device: String,
    pub output_device: String,
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
