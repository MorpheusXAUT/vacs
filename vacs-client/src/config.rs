use anyhow::Context;
use config::{Config, Environment, File};
use serde::Deserialize;

/// User-Agent string used for all HTTP requests.
pub static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub backend: BackendConfig,
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

pub fn project_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("app", "vacs", "vacs-client")
}
