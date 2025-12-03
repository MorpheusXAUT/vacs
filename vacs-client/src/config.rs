use crate::app::window::WindowProvider;
use crate::error::Error;
use crate::radio::push_to_talk::PushToTalkRadio;
use crate::radio::{DynRadio, RadioIntegration};
use anyhow::Context;
use config::{Config, Environment, File};
use keyboard_types::Code;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauri::{LogicalSize, PhysicalPosition, PhysicalSize};
use vacs_signaling::protocol::http::version::ReleaseChannel;
use vacs_signaling::protocol::http::webrtc::IceConfig;

/// User-Agent string used for all HTTP requests.
pub static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
pub const WS_LOGIN_TIMEOUT: Duration = Duration::from_secs(10);
pub const AUDIO_SETTINGS_FILE_NAME: &str = "audio.toml";
pub const CLIENT_SETTINGS_FILE_NAME: &str = "client.toml";
pub const STATIONS_SETTINGS_FILE_NAME: &str = "stations.toml";
pub const ENCODED_AUDIO_FRAME_BUFFER_SIZE: usize = 512;
pub const ICE_CONFIG_EXPIRY_LEEWAY: Duration = Duration::from_mins(15);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub backend: BackendConfig,
    pub audio: AudioConfig,
    #[serde(alias = "webrtc")] // support for old naming scheme
    pub ice: IceConfig,
    pub client: ClientConfig,
    pub stations: StationsConfig,
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
                        .join(AUDIO_SETTINGS_FILE_NAME)
                        .to_str()
                        .expect("Failed to get local config path"),
                )
                .required(false),
            )
            .add_source(File::with_name(AUDIO_SETTINGS_FILE_NAME).required(false))
            .add_source(
                File::with_name(
                    config_dir
                        .join(STATIONS_SETTINGS_FILE_NAME)
                        .to_str()
                        .expect("Failed to get local config path"),
                )
                .required(false),
            )
            .add_source(File::with_name(STATIONS_SETTINGS_FILE_NAME).required(false))
            .add_source(
                File::with_name(
                    config_dir
                        .join(CLIENT_SETTINGS_FILE_NAME)
                        .to_str()
                        .expect("Failed to get local config path"),
                )
                .required(false),
            )
            .add_source(File::with_name(CLIENT_SETTINGS_FILE_NAME).required(false))
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
            BackendEndpoint::IceConfig => &self.endpoints.ice_config,
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
    IceConfig,
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
    pub ice_config: String,
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
            ice_config: "/webrtc/ice-config".to_string(),
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
    pub fullscreen: bool,
    pub position: Option<PhysicalPosition<i32>>,
    pub size: Option<PhysicalSize<u32>>,
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
            fullscreen: false,
            position: None,
            size: None,
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

    pub fn default_window_size<P>(provider: &P) -> Result<PhysicalSize<u32>, Error>
    where
        P: WindowProvider + ?Sized,
    {
        Ok(LogicalSize::new(
            1000.0f64,
            if cfg!(target_os = "macos") {
                781.0f64
            } else {
                753.0f64
            },
        )
        .to_physical(provider.scale_factor()?))
    }

    pub fn update_window_state<P>(&mut self, provider: &P) -> Result<(), Error>
    where
        P: WindowProvider + ?Sized,
    {
        let window = provider.window()?;
        self.position = Some(window.position()?);
        self.size = Some(window.size()?);

        log::debug!(
            "Updating window position to {:?} and size to {:?}",
            self.position.unwrap(),
            self.size.unwrap()
        );
        Ok(())
    }

    pub fn restore_window_state<P>(&self, provider: &P) -> Result<(), Error>
    where
        P: WindowProvider + ?Sized,
    {
        let window = provider.window()?;

        log::debug!(
            "Restoring window position to {:?} and size to {:?}",
            self.position,
            self.size
        );

        if let Some(position) = self.position {
            for m in window
                .available_monitors()
                .context("Failed to get available monitors")?
            {
                let PhysicalPosition { x, y } = *m.position();
                let PhysicalSize { width, height } = *m.size();

                let left = x;
                let right = x + width as i32;
                let top = y;
                let bottom = y + height as i32;

                let size = self.size.unwrap_or(Self::default_window_size(&window)?);

                let intersects = [
                    (position.x, position.y),
                    (position.x + size.width as i32, position.y),
                    (position.x, position.y + size.height as i32),
                    (
                        position.x + size.width as i32,
                        position.y + size.height as i32,
                    ),
                ]
                .into_iter()
                .any(|(x, y)| x >= left && x < right && y >= top && y < bottom);

                if intersects {
                    window
                        .set_position(position)
                        .context("Failed to set main window position")?;
                    break;
                }
            }
        }

        if let Some(size) = self.size {
            window
                .set_size(size)
                .context("Failed to set main window size")?;

            #[cfg(target_os = "linux")]
            {
                log::debug!("Verifying correct window size after decorations apply");

                // This timeout is **absolutely crucial** as the window manager does not update the
                // window size immediately after a resize has been requested, but only after a short
                // delay. If we were to compare the window size immediately after resizing, we would
                // always receive the expected values, however, the window manager would still apply
                // decorations later, changing the actual size, which is then incorrectly persisted.
                // This will result in a short "flicker" of the window size, which we would optimally
                // hide by simply not showing the window until we're sure its size is correct. However,
                // since there's another bug that prevents the menu bar from being interactable if the
                // window is initialized hidden, which is even less desirable, we'll have to live with
                // the flicker for now.
                // Upstream tauri/tao issues related to this:
                // - https://github.com/tauri-apps/tao/issues/929
                // - https://github.com/tauri-apps/tao/pull/1055
                std::thread::sleep(Duration::from_millis(50));
                let actual_size = window.inner_size().context("Failed to get window size")?;

                let width_diff = actual_size.width.saturating_sub(size.width);
                let height_diff = actual_size.height.saturating_sub(size.height);

                if width_diff > 0 || height_diff > 0 {
                    log::warn!(
                        "Window size changed after decorations apply, expected: {size:?}, got: {actual_size:?}. Resizing again"
                    );
                    window
                        .set_size(PhysicalSize::new(
                            size.width.saturating_sub(width_diff),
                            size.height.saturating_sub(height_diff),
                        ))
                        .context("Failed to fix main window size")?;
                }
            }
        }

        Ok(())
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
    /// Create a radio integration instance based on the configured integration type.
    ///
    /// Returns `None` if the integration is not configured or if the emit key is not set.
    ///
    /// # Platform Limitation
    ///
    /// **Important**: Radio integration requires a functional `KeybindEmitter` to inject
    /// key presses into external applications. This works on Windows and macOS, but NOT
    /// on Linux where the emitter is a no-op stub due to Wayland's security model.
    ///
    /// On Linux, this method will successfully create a radio instance, but it will
    /// silently do nothing when `transmit()` is called.
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

/// Configuration for how stations are handled client-side.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationsConfig {
    pub selected_profile: String,
    /// Named profiles for different station filtering configurations.
    /// Users can switch between profiles in the UI.
    pub profiles: HashMap<String, StationsProfileConfig>,
}

impl Default for StationsConfig {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert("Default".to_string(), StationsProfileConfig::default());
        Self {
            selected_profile: "Default".to_string(),
            profiles,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendStationsConfig {
    pub selected_profile: String,
    pub profiles: HashMap<String, StationsProfileConfig>,
}

impl From<StationsConfig> for FrontendStationsConfig {
    fn from(stations_config: StationsConfig) -> Self {
        Self {
            selected_profile: stations_config.selected_profile,
            profiles: stations_config.profiles,
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PersistedStationsConfig {
    pub stations: StationsConfig,
}

impl From<StationsConfig> for PersistedStationsConfig {
    fn from(stations: StationsConfig) -> Self {
        Self { stations }
    }
}

/// Config profile for how stations are filtered, prioritized and displayed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationsProfileConfig {
    /// Optional list of callsign patterns to include.
    ///
    /// - If this list is empty, all stations are eligible to be shown (subject to `exclude`).
    /// - If this list is not empty, only stations matching at least one pattern are eligible to be shown.
    ///
    /// Glob syntax is supported: `"LO*"`, `"LOWW_*"`, `"*_APP"`, …
    /// Matching is case-insensitive.
    ///
    /// Example:
    ///   `["LO*", "EDDM_*", "EDMM_*"]`
    #[serde(default)]
    pub include: Vec<String>,

    /// Optional list of callsign patterns to exclude.
    ///
    /// - Stations matching any pattern here are never shown, even if they match an `include` rule.
    ///
    /// Glob syntax is supported: `"LO*"`, `"LOWW_*"`, `"*_APP"`, …
    /// Matching is case-insensitive.
    ///
    /// Example:
    ///   `["*_TWR", "*_GND", "*_DEL"]`
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Optional ordered list of callsign patterns used to assign priority.
    ///
    /// The *first* matching pattern in the list determines the station's
    /// priority bucket. Earlier entries = higher priority.
    ///
    /// Glob syntax is supported: `"LO*"`, `"LOWW_*"`, `"*_APP"`, …
    /// Matching is case-insensitive.
    ///
    /// Example:
    ///   `["LOVV_*", "LOWW_*_APP", "LOWW_*_TWR", "LOWW_*"]`
    #[serde(default)]
    pub priority: Vec<String>,

    /// Optional alias mapping of frequencies to custom display names.
    ///
    /// - If a station's frequency matches a key in this map, the corresponding display name will be
    ///   used instead of the one received from VATSIM.
    /// - **Important**: Display names should follow the same underscore-separated format as VATSIM
    ///   callsigns (e.g., `Station_Name_TYPE`) to ensure proper filtering, sorting and display.
    ///   The last part after the final underscore is used as the station type.
    /// - Frequency mapping is exact (no wildcard support).
    ///
    /// This is useful for:
    /// - Customizing station names if the VATSIM callsign doesn't match the sector's desired display name
    /// - Using local language or abbreviations (e.g., "Wien" instead of "LOWW")
    /// - Providing a "stable" list of stations even when relieve/personalized callsigns are used
    ///
    /// Example:
    /// ```toml
    /// [stations.profiles.Default.aliases]
    /// "132.600" = "AC_CTR"
    /// "124.400" = "FIC_CTR"
    /// ```
    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

impl Default for StationsProfileConfig {
    fn default() -> Self {
        Self {
            include: vec![],
            exclude: vec![],
            priority: vec![
                "*_FMP".to_string(),
                "*_CTR".to_string(),
                "*_APP".to_string(),
                "*_TWR".to_string(),
                "*_GND".to_string(),
            ],
            aliases: HashMap::new(),
        }
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
