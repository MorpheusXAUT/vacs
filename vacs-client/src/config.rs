use crate::error::Error;
use crate::radio::push_to_talk::PushToTalkRadio;
use crate::radio::{DynRadio, RadioIntegration};
use anyhow::Context;
use config::{Config, Environment, File};
use keyboard_types::Code;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
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
    pub radio: RadioConfig,
    pub auto_hangup_seconds: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            always_on_top: false,
            release_channel: ReleaseChannel::default(),
            signaling_auto_reconnect: true,
            transmit_config: TransmitConfig::default(),
            radio: RadioConfig::default(),
            auto_hangup_seconds: 60,
        }
    }
}

impl ClientConfig {
    pub fn max_signaling_reconnect_attempts(&self) -> u8 {
        if self.signaling_auto_reconnect { 8 } else { 0 }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub enum TransmitMode {
    #[default]
    VoiceActivation,
    PushToTalk,
    PushToMute,
    RadioIntegration,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransmitConfig {
    pub mode: TransmitMode,
    pub push_to_talk: Option<Code>,
    pub push_to_mute: Option<Code>,
    pub radio_push_to_talk: Option<Code>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrontendTransmitConfig {
    pub mode: TransmitMode,
    pub push_to_talk: Option<String>,
    pub push_to_mute: Option<String>,
    pub radio_push_to_talk: Option<String>,
}

impl From<TransmitConfig> for FrontendTransmitConfig {
    fn from(transmit_config: TransmitConfig) -> Self {
        Self {
            mode: transmit_config.mode,
            push_to_talk: transmit_config.push_to_talk.map(|c| c.to_string()),
            push_to_mute: transmit_config.push_to_mute.map(|c| c.to_string()),
            radio_push_to_talk: transmit_config.radio_push_to_talk.map(|c| c.to_string()),
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
                .map(|s| s.parse::<Code>())
                .transpose()
                .map_err(|_| Error::Other(Box::new(anyhow::anyhow!("Unrecognized key code: {}. Please report this error in our GitHub repository's issue tracker.", value.push_to_talk.unwrap_or_default()))))?,
            push_to_mute: value
                .push_to_mute
                .as_ref()
                .map(|s| s.parse::<Code>())
                .transpose()
                .map_err(|_| Error::Other(Box::new(anyhow::anyhow!("Unrecognized key code: {}. Please report this error in our GitHub repository's issue tracker.", value.push_to_mute.unwrap_or_default()))))?,
            radio_push_to_talk: value
                .radio_push_to_talk
                .as_ref()
                .map(|s| s.parse::<Code>())
                .transpose()
                .map_err(|_| Error::Other(Box::new(anyhow::anyhow!("Unrecognized key code: {}. Please report this error in our GitHub repository's issue tracker.", value.radio_push_to_talk.unwrap_or_default()))))?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadioConfig {
    pub integration: RadioIntegration,
    pub audio_for_vatsim: Option<AudioForVatsimRadioConfig>,
    pub track_audio: Option<TrackAudioRadioConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioForVatsimRadioConfig {
    pub emit: Option<Code>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrackAudioRadioConfig {
    pub emit: Option<Code>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrontendRadioConfig {
    pub integration: RadioIntegration,
    pub audio_for_vatsim: Option<FrontendAudioForVatsimRadioConfig>,
    pub track_audio: Option<FrontendTrackAudioRadioConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrontendAudioForVatsimRadioConfig {
    pub emit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrontendTrackAudioRadioConfig {
    pub emit: Option<String>,
}

impl RadioConfig {
    pub fn radio(&self) -> Result<Option<DynRadio>, Error> {
        match self.integration {
            RadioIntegration::AudioForVatsim => {
                let Some(config) = self.audio_for_vatsim.as_ref() else {
                    return Ok(None);
                };
                let Some(emit) = config.emit else {
                    return Ok(None);
                };
                log::debug!("Initializing AudioForVatsim radio integration");
                let radio = PushToTalkRadio::new(emit).map_err(Error::from)?;
                Ok(Some(Arc::new(radio)))
            }
            RadioIntegration::TrackAudio => {
                let Some(config) = self.track_audio.as_ref() else {
                    return Ok(None);
                };
                let Some(emit) = config.emit else {
                    return Ok(None);
                };
                log::debug!("Initializing TrackAudio radio integration");
                let radio = PushToTalkRadio::new(emit).map_err(Error::from)?;
                Ok(Some(Arc::new(radio)))
            }
        }
    }
}

impl From<RadioConfig> for FrontendRadioConfig {
    fn from(radio_integration: RadioConfig) -> Self {
        Self {
            integration: radio_integration.integration,
            audio_for_vatsim: radio_integration.audio_for_vatsim.map(|c| c.into()),
            track_audio: radio_integration.track_audio.map(|c| c.into()),
        }
    }
}

impl From<AudioForVatsimRadioConfig> for FrontendAudioForVatsimRadioConfig {
    fn from(value: AudioForVatsimRadioConfig) -> Self {
        Self {
            emit: value.emit.map(|c| c.to_string()),
        }
    }
}

impl From<TrackAudioRadioConfig> for FrontendTrackAudioRadioConfig {
    fn from(value: TrackAudioRadioConfig) -> Self {
        Self {
            emit: value.emit.map(|c| c.to_string()),
        }
    }
}

impl TryFrom<FrontendRadioConfig> for RadioConfig {
    type Error = Error;

    fn try_from(value: FrontendRadioConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            integration: value.integration,
            audio_for_vatsim: value.audio_for_vatsim.map(|c| c.try_into()).transpose()?,
            track_audio: value.track_audio.map(|c| c.try_into()).transpose()?,
        })
    }
}

impl TryFrom<FrontendAudioForVatsimRadioConfig> for AudioForVatsimRadioConfig {
    type Error = Error;

    fn try_from(value: FrontendAudioForVatsimRadioConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            emit: value
                .emit
                .as_ref()
                .map(|s| s.parse::<Code>())
                .transpose()
                .map_err(|_| Error::Other(Box::new(anyhow::anyhow!("Unrecognized key code: {}. Please report this error in our GitHub repository's issue tracker.", value.emit.unwrap_or_default()))))?,
        })
    }
}

impl TryFrom<FrontendTrackAudioRadioConfig> for TrackAudioRadioConfig {
    type Error = Error;

    fn try_from(value: FrontendTrackAudioRadioConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            emit: value
                .emit
                .as_ref()
                .map(|s| s.parse::<Code>())
                .transpose()
                .map_err(|_| Error::Other(Box::new(anyhow::anyhow!("Unrecognized key code: {}. Please report this error in our GitHub repository's issue tracker.", value.emit.unwrap_or_default()))))?,
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
