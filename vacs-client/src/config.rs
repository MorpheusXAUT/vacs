use anyhow::Context;
use config::{Config, Environment, File};
use serde::Deserialize;

/// User-Agent string used for all HTTP requests.
pub static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub backend: BackendConfig,
}

impl AppConfig {
    pub fn parse() -> anyhow::Result<AppConfig> {
        Config::builder()
            .set_default("backend.base_url", "http://localhost:3000")?
            .set_default("backend.ws_url", "ws://localhost:3000/ws")?
            .set_default("backend.endpoints.init_auth", "/auth/vatsim")?
            .set_default("backend.endpoints.exchange_code", "/auth/vatsim/callback")?
            .set_default("backend.endpoints.ws_token", "/ws/token")?
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
            .add_source(Environment::with_prefix("vacs_client"))
            .build()
            .context("Failed to build config")?
            .try_deserialize()
            .context("Failed to deserialize config")
    }
}

#[derive(Debug, Deserialize)]
pub struct BackendConfig {
    pub base_url: String,
    pub ws_url: String,
    pub endpoints: BackendEndpointsConfigs,
}

impl BackendConfig {
    pub fn endpoint_url(&self, endpoint: BackendEndpoint) -> String {
        let path = match endpoint {
            BackendEndpoint::InitAuth => &self.endpoints.init_auth,
            BackendEndpoint::ExchangeCode => &self.endpoints.exchange_code,
            BackendEndpoint::WsToken => &self.endpoints.ws_token,
        };
        format!("{}{}", self.base_url, path)
    }
}

pub enum BackendEndpoint {
    InitAuth,
    ExchangeCode,
    WsToken,
}

#[derive(Debug, Deserialize)]
pub struct BackendEndpointsConfigs {
    pub init_auth: String,
    pub exchange_code: String,
    pub ws_token: String,
}

pub fn project_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("app", "vacs", "vacs-client")
}
