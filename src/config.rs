use log::LevelFilter;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct AppConfig {
    pub api: ApiConfig,
    pub audio: AudioConfig,
    pub webrtc: WebrtcConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct ApiConfig {
    pub url: String,
    pub key: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct AudioConfig {
    pub input: AudioDeviceConfig,
    pub output: AudioDeviceConfig,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct AudioDeviceConfig {
    pub host_name: Option<String>,
    pub device_name: Option<String>,
    pub channels: u16,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct WebrtcConfig {
    pub ice_servers: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct LoggingConfig {
    pub level: LevelFilter,
}
