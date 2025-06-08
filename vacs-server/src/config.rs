use serde::Deserialize;
use std::time::Duration;

pub const BROADCAST_CHANNEL_CAPACITY: usize = 100;
pub const CLIENT_CHANNEL_CAPACITY: usize = 100;
pub const SERVER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            auth: AuthConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:3000".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub login_flow_timeout_millis: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            login_flow_timeout_millis: 10000,
        }
    }
}
