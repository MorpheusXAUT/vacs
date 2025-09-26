use crate::error::Error;
use anyhow::Context;
use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tauri_plugin_global_shortcut::{Code, Shortcut};
use vacs_signaling::protocol::http::version::ReleaseChannel;
use vacs_webrtc::config::WebrtcConfig;

/// User-Agent string used for all HTTP requests.
pub static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
pub const WS_LOGIN_TIMEOUT: Duration = Duration::from_secs(10);
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
            BackendEndpoint::VersionUpdateCheck => &self.endpoints.version_update_check,
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
    VersionUpdateCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendEndpointsConfigs {
    pub init_auth: String,
    pub exchange_code: String,
    pub user_info: String,
    pub logout: String,
    pub ws_token: String,
    pub terminate_ws_session: String,
    pub version_update_check: String,
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
            version_update_check: "/version/update?version={{current_version}}&target={{target}}&arch={{arch}}&bundle_type={{bundle_type}}&channel={{channel}}".to_string(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub always_on_top: bool,
    pub release_channel: ReleaseChannel,
    pub signaling_auto_reconnect: bool,
    pub transmit_config: TransmitConfig,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            always_on_top: false,
            release_channel: ReleaseChannel::default(),
            signaling_auto_reconnect: true,
            transmit_config: TransmitConfig::default(),
        }
    }
}

impl ClientConfig {
    pub fn max_signaling_reconnect_attempts(&self) -> u8 {
        if self.signaling_auto_reconnect { 8 } else { 0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub enum TransmitMode {
    #[default]
    VoiceActivation,
    PushToTalk,
    PushToMute,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransmitConfig {
    pub mode: TransmitMode,
    #[serde(with = "shortcut_code_opt")]
    pub push_to_talk: Option<Shortcut>,
    #[serde(with = "shortcut_code_opt")]
    pub push_to_mute: Option<Shortcut>,
}

mod shortcut_code_opt {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(value: &Option<Shortcut>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let as_code: Option<Code> = value.as_ref().map(|s| s.key);
        as_code.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Shortcut>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt_code = Option::<Code>::deserialize(deserializer)?;
        Ok(opt_code.map(|code| Shortcut::new(None, code)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrontendTransmitConfig {
    pub mode: TransmitMode,
    pub push_to_talk: Option<String>,
    pub push_to_mute: Option<String>,
}

impl From<TransmitConfig> for FrontendTransmitConfig {
    fn from(transmit_config: TransmitConfig) -> Self {
        Self {
            mode: transmit_config.mode,
            push_to_talk: transmit_config.push_to_talk.map(|s| s.key.to_string()),
            push_to_mute: transmit_config.push_to_mute.map(|s| s.key.to_string()),
        }
    }
}

impl TryFrom<FrontendTransmitConfig> for TransmitConfig {
    type Error = Error;

    fn try_from(value: FrontendTransmitConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            mode: value.mode,
            push_to_talk: value
                .push_to_talk
                .as_ref()
                .map(|s| Code::from_str(s).map(|code| Shortcut::new(None, code)))
                .transpose()
                .map_err(|_| Error::Other(Box::new(anyhow::anyhow!("Unrecognized key code: {}. Please report this error in our GitHub repository's issue tracker.", value.push_to_talk.unwrap_or_default()))))?,
            push_to_mute: value
                .push_to_mute
                .as_ref()
                .map(|s| Code::from_str(s).map(|code| Shortcut::new(None, code)))
                .transpose()
                .map_err(|_| Error::Other(Box::new(anyhow::anyhow!("Unrecognized key code: {}. Please report this error in our GitHub repository's issue tracker.", value.push_to_mute.unwrap_or_default()))))?,
        })
    }
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
