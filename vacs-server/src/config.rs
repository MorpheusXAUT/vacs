use anyhow::Context;
use axum_client_ip::ClientIpSource;
use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

pub const BROADCAST_CHANNEL_CAPACITY: usize = 100;
pub const CLIENT_CHANNEL_CAPACITY: usize = 100;
pub const CLIENT_WEBSOCKET_TASK_CHANNEL_CAPACITY: usize = 100;
pub const CLIENT_WEBSOCKET_PING_INTERVAL: Duration = Duration::from_secs(10);
pub const CLIENT_WEBSOCKET_PONG_TIMEOUT: Duration = Duration::from_secs(30);
pub const SERVER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub redis: RedisConfig,
    pub session: SessionConfig,
    pub auth: AuthConfig,
    pub vatsim: VatsimConfig,
    pub updates: UpdatesConfig,
}

impl AppConfig {
    pub fn parse() -> anyhow::Result<Self> {
        let config = Config::builder()
            .add_source(Config::try_from(&AppConfig::default())?)
            .add_source(File::with_name(config_file_path("config.toml")?.as_str()).required(false))
            .add_source(File::with_name("config.toml").required(false))
            .add_source(
                Environment::with_prefix("vacs")
                    .separator("-")
                    .try_parsing(true),
            )
            .build()
            .context("Failed to build config")?
            .try_deserialize::<Self>()
            .context("Failed to deserialize config")?;

        if config.auth.oauth.client_id.is_empty() {
            anyhow::bail!("OAuth client ID is empty");
        } else if config.auth.oauth.client_secret.is_empty() {
            anyhow::bail!("OAuth client secret is empty");
        } else if config.session.signing_key.is_empty() {
            anyhow::bail!("Session signing key is empty");
        }

        Ok(config)
    }
}

pub fn config_file_path(file_name: impl AsRef<Path>) -> anyhow::Result<String> {
    Ok(Path::new("/etc")
        .join(env!("CARGO_PKG_NAME").to_lowercase())
        .join(file_name)
        .to_str()
        .context("Failed to build config file path")?
        .to_string())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub client_ip_source: ClientIpSource,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:3000".to_string(),
            client_ip_source: ClientIpSource::ConnectInfo,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RedisConfig {
    pub addr: String,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            addr: "redis://127.0.0.1:6379".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionConfig {
    pub secure: bool,
    pub http_only: bool,
    pub expiry_secs: i64,
    pub signing_key: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            secure: true,
            http_only: true,
            expiry_secs: 604800, // 7 days
            signing_key: "".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthConfig {
    pub login_flow_timeout_millis: u64,
    pub oauth: OAuthConfig,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            login_flow_timeout_millis: 10000,
            oauth: OAuthConfig::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthConfig {
    pub auth_url: String,
    pub token_url: String,
    pub redirect_url: String,
    pub client_id: String,
    pub client_secret: String,
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            auth_url: "https://auth-dev.vatsim.net/oauth/authorize".to_string(),
            token_url: "https://auth-dev.vatsim.net/oauth/token".to_string(),
            redirect_url: "vacs://auth/vatsim/callback".to_string(),
            client_id: "".to_string(),
            client_secret: "".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VatsimConfig {
    pub user_service: VatsimUserServiceConfig,
    pub require_active_connection: bool,
    pub slurper_base_url: String,
    pub data_feed_url: String,
    pub controller_update_interval: Duration,
}

impl Default for VatsimConfig {
    fn default() -> Self {
        Self {
            user_service: Default::default(),
            require_active_connection: true,
            slurper_base_url: "https://slurper.vatsim.net".to_string(),
            data_feed_url: "https://data.vatsim.net/v3/vatsim-data.json".to_string(),
            controller_update_interval: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VatsimUserServiceConfig {
    pub user_details_endpoint_url: String,
}

impl Default for VatsimUserServiceConfig {
    fn default() -> Self {
        Self {
            user_details_endpoint_url: "https://auth-dev.vatsim.net/api/user".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdatesConfig {
    pub release_manifest_path: String,
    pub policy_path: String,
}

impl Default for UpdatesConfig {
    fn default() -> Self {
        Self {
            release_manifest_path: config_file_path("releases.toml")
                .expect("Failed to build release manifest path"),
            policy_path: config_file_path("release_policy.toml")
                .expect("Failed to build policy path"),
        }
    }
}
