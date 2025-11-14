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

/// Top-level configuration for the vacs client.
///
/// This is assembled from:
///
/// 1. Built-in defaults (`AppConfig::default()`),
/// 2. `config.toml` in the config directory,
/// 3. `config.toml` in the current working directory,
/// 4. `audio.toml` / `client.toml` in the config directory,
/// 5. `audio.toml` in the current working directory,
/// 6. Environment variables with the `VACS_CLIENT_` prefix.
///
/// Later sources override earlier ones.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// Backend/signaling (HTTP/WebSocket) endpoints and timeouts.
    pub backend: BackendConfig,
    /// Local audio device and volume configuration.
    pub audio: AudioConfig,
    /// WebRTC configuration for call setup.
    pub webrtc: WebrtcConfig,
    /// Client UI/behavior configuration (transmit mode, radio integration, etc.).
    pub client: ClientConfig,
}

impl AppConfig {
    /// Build a complete [`AppConfig`] from defaults, config files and environment.
    ///
    /// * `config_dir` is typically the platform-specific config directory
    ///   (e.g. `~/.config/vacs-client` on Linux).
    ///
    /// Order of precedence (later overrides earlier):
    /// 1. `AppConfig::default()`
    /// 2. `${config_dir}/config.toml`
    /// 3. `./config.toml`
    /// 4. `${config_dir}/audio.toml`
    /// 5. `${config_dir}/client.toml`
    /// 6. `./audio.toml`
    /// 7. Environment variables with prefix `VACS_CLIENT_`
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

/// Configuration for the signaling/backend server.
///
/// Controls where OAuth, version checks and the WebSocket connection
/// are sent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    /// Base URL for all HTTP endpoints. Defaults to `https://vacs.gusch.jetzt` for release and `https://vacs-dev.gusch.jetzt` for dev builds.
    pub base_url: String,
    /// Base URL for the WebSocket endpoint, including protocol. Defaults to `wss://vacs.gusch.jetzt/ws` for release and `wss://vacs-dev.gusch.jetzt/ws` for dev builds.
    pub ws_url: String,
    /// Paths for individual backend endpoints relative to `base_url`.
    pub endpoints: BackendEndpointsConfigs,
    /// HTTP request timeout in milliseconds for backend calls.
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
    /// Build a full URL (base + path) for the given [`BackendEndpoint`].
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

/// Logical backend endpoints used by the client.
///
/// These are mapped to concrete URL paths via [`BackendEndpointsConfigs`].
pub enum BackendEndpoint {
    InitAuth,
    ExchangeCode,
    UserInfo,
    Logout,
    WsToken,
    TerminateWsSession,
    VersionUpdateCheck,
}

/// Path configuration for backend endpoints.
///
/// All values are relative to [`BackendConfig::base_url`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendEndpointsConfigs {
    /// Path for starting the VATSIM OAuth flow.
    /// Default: `/auth/vatsim`
    pub init_auth: String,
    /// Path for exchanging the OAuth callback code for a token.
    /// Default: `/auth/vatsim/callback`
    pub exchange_code: String,
    /// Path for retrieving the authenticated user's info.
    /// Default: `/auth/user`
    pub user_info: String,
    /// Path for logging out and clearing sessions on the backend.
    /// Default: `/auth/logout`
    pub logout: String,
    /// Path for requesting a short-lived WebSocket login token.
    /// Default: `/ws/token`
    pub ws_token: String,
    /// Path used to instruct the backend to terminate an existing WS session.
    /// Default: `/ws`
    pub terminate_ws_session: String,
    /// Path for version update checks.
    ///
    /// The default contains template placeholders:
    /// `{{current_version}}`, `{{target}}`, `{{arch}}`, `{{bundle_type}}`, `{{channel}}`,
    /// which the client will replace when constructing the URL.
    /// Default: `/version/update?version={{current_version}}&target={{target}}&arch={{arch}}&bundle_type={{bundle_type}}&channel={{channel}}`
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

/// Local audio device and mixer configuration.
///
/// All volume values are `0.0..=1.0` and represent a user-visible slider.
/// The corresponding `*_amp` values are linear gain factors applied in
/// addition to those sliders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Name of the audio backend host to use (CPAL host).
    ///
    /// `None` uses the system default host.
    /// Default: `None`
    pub host_name: Option<String>,
    /// Exact name of the input audio device.
    ///
    /// `None` uses the default input device for the chosen host.
    /// Default: `None`
    pub input_device_name: Option<String>,
    /// Exact name of the output audio device.
    ///
    /// `None` uses the default output device for the chosen host.
    /// Default: `None`
    pub output_device_name: Option<String>,
    /// Input volume slider value (0.0–1.0) before additional amplification.
    /// Default: `0.5`
    pub input_device_volume: f32,
    /// Additional input amplification factor (linear gain).
    ///
    /// Defaults to `4.0` to compensate for typically low microphone levels.
    /// Not changeable via UI.
    pub input_device_volume_amp: f32,
    /// Output volume slider value (0.0–1.0).
    /// Default: `0.5`
    pub output_device_volume: f32,
    /// Additional output amplification factor (linear gain).
    ///
    /// Defaults to `2.0` to give some headroom above the slider.
    /// Not changeable via UI.
    pub output_device_volume_amp: f32,
    /// Volume of UI click sounds (0.0–1.0).
    /// Default: `0.5`
    pub click_volume: f32,
    /// Volume of UI click sounds (0.0–1.0).
    /// Default: `0.5`
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

/// Wrapper used when persisting only the [`AudioConfig`] section to `audio.toml`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct PersistedAudioConfig {
    pub audio: AudioConfig,
}

impl From<AudioConfig> for PersistedAudioConfig {
    fn from(audio: AudioConfig) -> Self {
        Self { audio }
    }
}

/// Client-side behavior, UI and transmit configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Whether the main window should stay on top of other windows.
    /// Default: `false`
    pub always_on_top: bool,
    /// Release channel used for update checks (e.g., stable, beta, dev).
    ///
    /// See [`vacs_signaling::protocol::http::version::ReleaseChannel`].
    /// Default: `stable`
    pub release_channel: ReleaseChannel,
    /// Automatically reconnect to the signaling backend when the WebSocket
    /// connection is lost.
    /// Default: `true`
    pub signaling_auto_reconnect: bool,
    /// Configuration for how the user transmits audio (VOX, PTT, PTM).
    pub transmit_config: TransmitConfig,
    /// Configuration for the radio components of vacs, such as the external radio integration.
    pub radio: RadioConfig,
    /// Number of seconds after which an unanswered outgoing call is automatically cancelled.
    /// Default: `60`
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

/// How the client decides when to transmit audio.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub enum TransmitMode {
    /// Always transmit audio while in a call.
    #[default]
    VoiceActivation,
    /// Only transmit audio when the user presses a key while in a call.
    PushToTalk,
    /// Always transmit audio while in a call, but mute audio when the user presses a key.
    PushToMute,
    /// Integrate with an external radio application (AFV/TrackAudio).
    ///
    /// In this mode, audio is only transmitted when the user presses a key while in a call.
    RadioIntegration,
}

/// Audio transmission config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransmitConfig {
    /// Selected transmit mode.
    pub mode: TransmitMode,
    /// Optional PTT key used when `mode` is set to [`TransmitMode::PushToTalk`].
    /// This uses [`keyboard_types::Code`] (physical key location) to remain layout-independent.
    pub push_to_talk: Option<Code>,
    /// Optional PTM key used when `mode` is set to [`TransmitMode::PushToMute`].
    /// This uses [`keyboard_types::Code`] (physical key location) to remain layout-independent.
    pub push_to_mute: Option<Code>,
    /// Optional PTT key used when `mode` is set to [`TransmitMode::RadioIntegration`].
    /// This uses [`keyboard_types::Code`] (physical key location) to remain layout-independent.
    ///
    /// This is the key the user presses to either talk in a call or on frequency.
    pub radio_push_to_talk: Option<Code>,
}

/// Transmit configuration representation used by the frontend.
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

/// Configuration for radio components.
///
/// The actual integration is selected via [`RadioIntegration`], with
/// optional backend-specific nested configs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadioConfig {
    /// Selected radio integration backend (None, AudioForVatsim, TrackAudio, …).
    pub integration: RadioIntegration,
    /// Optional configuration for the AudioForVATSIM integration.
    pub audio_for_vatsim: Option<AudioForVatsimRadioConfig>,
    /// Optional configuration for the TrackAudio integration.
    pub track_audio: Option<TrackAudioRadioConfig>,
}

/// Configuration specific to the AudioForVATSIM radio integration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioForVatsimRadioConfig {
    /// Optional key that vacs will press as PTT in the AFV client.
    /// This uses [`keyboard_types::Code`] (physical key location) to remain layout-independent.
    ///
    /// If `None`, the AFV integration is disabled even if selected.
    pub emit: Option<Code>,
}

/// Configuration specific to the TrackAudio radio integration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrackAudioRadioConfig {
    /// Optional key that vacs will press as PTT in the TrackAudio client.
    /// This uses [`keyboard_types::Code`] (physical key location) to remain layout-independent.
    ///
    /// If `None`, the TrackAudio integration is disabled even if selected.
    pub emit: Option<Code>,
}

/// Radio configuration representation used by the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrontendRadioConfig {
    pub integration: RadioIntegration,
    pub audio_for_vatsim: Option<FrontendAudioForVatsimRadioConfig>,
    pub track_audio: Option<FrontendTrackAudioRadioConfig>,
}

/// AFV radio integration configuration representation used by the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrontendAudioForVatsimRadioConfig {
    pub emit: Option<String>,
}

/// TrackAudio radio integration configuration representation used by the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrontendTrackAudioRadioConfig {
    pub emit: Option<String>,
}

impl RadioConfig {
    /// Initialize the configured radio integration, if any.
    ///
    /// Returns:
    /// * `Ok(Some(radio))` if a valid integration is configured,
    /// * `Ok(None)` if the integration is disabled or incomplete,
    /// * `Err` if initialization fails.
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

/// Wrapper used when persisting only the [`ClientConfig`] section to `client.toml`.
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
