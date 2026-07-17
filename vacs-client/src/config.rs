use crate::app::window::WindowProvider;
use crate::error::Error;
use crate::keybinds::KeybindsError;
use crate::playback::PlaybackConfig;
use crate::radio::push_to_talk::PushToTalkRadio;
use crate::radio::track_audio::TrackAudioRadio;
use crate::radio::{DynRadio, RadioIntegration};
use crate::remote::RemoteConfig;
use anyhow::Context;
use config::{Config, Environment, File};
use keyboard_types::Code;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, LogicalSize, PhysicalPosition, PhysicalSize};
use vacs_signaling::protocol::http::version::ReleaseChannel;
use vacs_signaling::protocol::http::webrtc::IceConfig;
use vacs_signaling::protocol::profile::client_page::{
    ClientGroupMode, ClientPageConfig, FrequencyDisplayMode,
};
use vacs_signaling::protocol::vatsim::{ClientId, PositionId};

#[cfg(target_os = "linux")]
use crate::platform::Platform;

/// User-Agent string used for all HTTP requests.
pub static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
pub const WS_LOGIN_TIMEOUT: Duration = Duration::from_secs(10);
pub const DEFAULT_SETTINGS_FILE_NAME: &str = "config.toml";
pub const AUDIO_SETTINGS_FILE_NAME: &str = "audio.toml";
pub const CLIENT_SETTINGS_FILE_NAME: &str = "client.toml";
pub const CLIENT_PAGE_SETTINGS_FILE_NAME: &str = "client_page.toml";
pub const ENCODED_AUDIO_FRAME_BUFFER_SIZE: usize = 512;
pub const ICE_CONFIG_EXPIRY_LEEWAY: Duration = Duration::from_mins(15);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub backend: BackendConfig,
    pub audio: AudioConfig,
    #[serde(alias = "webrtc")] // support for old naming scheme
    pub ice: IceConfig,
    pub client: ClientConfig,
    #[serde(default)]
    pub client_page: ClientPageSettings,
}

impl AppConfig {
    pub fn parse(config_dir: &Path) -> anyhow::Result<Self> {
        let mut builder = Config::builder()
            .add_source(Config::try_from(&AppConfig::default())?)
            .add_source(
                File::with_name(
                    config_dir
                        .join(DEFAULT_SETTINGS_FILE_NAME)
                        .to_str()
                        .expect("Failed to get local config path"),
                )
                .required(false),
            )
            .add_source(File::with_name(DEFAULT_SETTINGS_FILE_NAME).required(false))
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
                        .join(CLIENT_PAGE_SETTINGS_FILE_NAME)
                        .to_str()
                        .expect("Failed to get local config path"),
                )
                .required(false),
            )
            .add_source(File::with_name(CLIENT_PAGE_SETTINGS_FILE_NAME).required(false))
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
            .add_source(Environment::with_prefix("vacs_client"));

        let preliminary_config: AppConfig = builder
            .build_cloned()
            .context("Failed to build preliminary config")?
            .try_deserialize()
            .context("Failed to deserialize preliminary config")?;

        if let Some(extra_client_page_config) = preliminary_config.client.extra_client_page_config {
            log::info!("Loading extra client page config from {extra_client_page_config}");
            builder = builder
                .add_source(File::with_name(&extra_client_page_config).required(false))
                .add_source(Environment::with_prefix("vacs_client"));
        }

        let mut config: AppConfig = builder
            .build()
            .context("Failed to build config")?
            .try_deserialize()
            .context("Failed to deserialize config")?;

        // Migrate old transmit config to new radio config, if mode was not RadioIntegration
        if let Some(was_radio_integration) = config.client.transmit_config.was_radio_integration
            && !was_radio_integration
        {
            config.client.radio.integration = None;
        }

        Ok(config)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub base_url: String,
    pub ws_url: String,
    pub endpoints: BackendEndpointsConfigs,
    pub timeout_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dev_position_id: Option<PositionId>,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            base_url: if cfg!(debug_assertions) || cfg!(feature = "rc") {
                "https://dev.vacs.network"
            } else {
                "https://vacs.network"
            }
            .to_string(),
            ws_url: if cfg!(debug_assertions) || cfg!(feature = "rc") {
                "wss://dev.vacs.network/ws"
            } else {
                "wss://vacs.network/ws"
            }
            .to_string(),
            endpoints: BackendEndpointsConfigs::default(),
            timeout_ms: 2000,
            dev_position_id: None,
        }
    }
}

impl BackendConfig {
    pub fn endpoint_url(&self, endpoint: &BackendEndpoint) -> String {
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

impl BackendEndpoint {
    pub const fn timeout(&self) -> Option<Duration> {
        match self {
            Self::ExchangeCode => Some(Duration::from_secs(2)),
            _ => None,
        }
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_device_id: Option<String>, // Stable device ID for reliable matching, None means default device
    pub output_device_name: Option<String>, // None means default device
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_device_id: Option<String>, // Stable device ID for reliable matching, None means default device
    pub speaker_enabled: bool,
    pub speaker_device_name: Option<String>, // None means default device
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker_device_id: Option<String>, // Stable device ID for reliable matching, None means default device
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
            input_device_id: None,
            output_device_name: None,
            output_device_id: None,
            speaker_enabled: false,
            speaker_device_name: None,
            speaker_device_id: None,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<PhysicalPosition<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<PhysicalSize<u32>>,
    pub release_channel: ReleaseChannel,
    pub signaling_auto_reconnect: bool,
    pub transmit_config: TransmitConfig,
    pub radio: RadioConfig,
    pub auto_hangup_seconds: u64,
    /// List of peer IDs (CIDs) that should be ignored by the client.
    ///
    /// Any incoming calls initiated by a CID in this list will be silently ignored
    /// by the client. This does **not** completely block communications with ignored
    /// parties as the (local) user can still actively initiate calls to them.
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub ignored: HashSet<ClientId>,
    #[serde(default)]
    pub keybinds: KeybindsConfig,
    #[serde(default)]
    pub call: CallConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_client_page_config: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_client_page_config: Option<String>,
    pub test_profile_watcher_delay_ms: u64,
    #[serde(default)]
    pub remote: RemoteConfig,
    #[serde(default)]
    pub playback: PlaybackConfig,
    #[serde(default = "default_zoom_level")]
    pub zoom_level: f64,
    #[serde(default)]
    pub clock_mode: ClockMode,
    #[serde(default)]
    pub cpl_mode: CplMode,
}

fn default_zoom_level() -> f64 {
    1.0f64
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
            ignored: HashSet::new(),
            keybinds: KeybindsConfig::default(),
            call: CallConfig::default(),
            selected_client_page_config: None,
            extra_client_page_config: None,
            test_profile_watcher_delay_ms: 500,
            remote: RemoteConfig::default(),
            playback: PlaybackConfig::default(),
            zoom_level: 1.0f64,
            clock_mode: ClockMode::default(),
            cpl_mode: CplMode::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ClockMode {
    #[default]
    Realtime,
    Relaxed,
    Day,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CplMode {
    #[default]
    Original,
    Fast,
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
        if window.is_minimized().unwrap_or(false) || window.is_maximized().unwrap_or(false) {
            log::debug!("Window is minimized or maximized, skipping window state update");
            return Ok(());
        }

        let size = window.size()?;
        if size.width == 0 || size.height == 0 {
            log::debug!("Window size is 0, skipping window state update");
            return Ok(());
        }

        let position = window.position()?;

        self.position = Some(position);
        self.size = Some(size);

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

        if let Some(mut size) = self.size {
            if size.width == 0 || size.height == 0 {
                log::warn!("Window size {size:?} is 0, restoring default size");
                size = Self::default_window_size(&window)?;
            }

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
pub enum CallMicMode {
    #[default]
    VoiceActivation,
    PushToTalk,
    PushToMute,
}

/// Configuration for the transmission mode and associated keybinds.
#[derive(Debug, Clone, Serialize, Default)]
pub struct TransmitConfig {
    /// The transmit mode to use.
    pub call_mic_mode: CallMicMode,
    /// Key code for Push-to-Talk mode.
    /// Required if mode is `PushToTalk`.
    pub push_to_talk: Option<Code>,
    /// Key code for Push-to-Mute mode.
    /// Required if mode is `PushToMute`.
    pub push_to_mute: Option<Code>,
    /// Key code for Radio PTT.
    pub radio_push_to_talk: Option<Code>,
    #[serde(skip)]
    pub was_radio_integration: Option<bool>,
}

impl TransmitConfig {
    #[inline]
    pub fn active_call_code(&self) -> Option<Code> {
        #[cfg(target_os = "linux")]
        if matches!(Platform::get(), Platform::LinuxWayland) {
            // Wayland Code Mapping Strategy:
            //
            // On Wayland, shortcuts are configured at the OS level via the XDG Global Shortcuts
            // portal. The portal allows complex key combinations (e.g., Ctrl+Alt+Shift+P) that
            // cannot be represented as a single keyboard_types::Code.
            //
            // To work around this, we map each transmit mode to a unique, unlikely-to-be-pressed
            // function key (F33-F35). These keys don't exist on most keyboards, so there's no
            // conflict with user input. When the portal activates a shortcut, we emit the
            // corresponding F-key code, and the rest of the keybind engine works unchanged.
            //
            // This effectively overrides the user-configured codes in the config file on Wayland,
            // since the actual key binding is managed by the desktop environment.
            let code = match self.call_mic_mode {
                CallMicMode::VoiceActivation => None,
                CallMicMode::PushToTalk => Some(Code::F33),
                CallMicMode::PushToMute => Some(Code::F34),
            };
            log::trace!(
                "Using portal shortcut code {code:?} for call mic mode {:?}",
                self.call_mic_mode
            );
            return code;
        }

        match self.call_mic_mode {
            CallMicMode::VoiceActivation => None,
            CallMicMode::PushToTalk => self.push_to_talk,
            CallMicMode::PushToMute => self.push_to_mute,
        }
    }

    #[inline]
    pub async fn active_radio_code(&self, enabled: bool) -> Option<Code> {
        if !enabled {
            return None;
        }

        #[cfg(target_os = "linux")]
        if matches!(Platform::get(), Platform::LinuxWayland) {
            use crate::keybinds::runtime;

            // Wayland Code Mapping Strategy:
            //
            // On Wayland, shortcuts are configured at the OS level via the XDG Global Shortcuts
            // portal. The portal allows complex key combinations (e.g., Ctrl+Alt+Shift+P) that
            // cannot be represented as a single keyboard_types::Code.
            //
            // To work around this, we map each transmit mode to a unique, unlikely-to-be-pressed
            // function key (F33-F35). These keys don't exist on most keyboards, so there's no
            // conflict with user input. When the portal activates a shortcut, we emit the
            // corresponding F-key code, and the rest of the keybind engine works unchanged.
            //
            // This effectively overrides the user-configured codes in the config file on Wayland,
            // since the actual key binding is managed by the desktop environment.
            let code = match self.call_mic_mode {
                CallMicMode::VoiceActivation => Some(Code::F35),
                CallMicMode::PushToTalk => {
                    if runtime::is_portal_shortcut_bound(runtime::PortalShortcutId::RadioPushToTalk)
                        .await
                    {
                        Some(Code::F35)
                    } else {
                        Some(Code::F33)
                    }
                }
                CallMicMode::PushToMute => Some(Code::F34),
            };
            log::trace!(
                "Using portal shortcut code {code:?} for call mic mode {:?}",
                self.call_mic_mode
            );
            return code;
        }

        match self.call_mic_mode {
            CallMicMode::VoiceActivation => self.radio_push_to_talk,
            CallMicMode::PushToTalk => self.radio_push_to_talk.or_else(|| self.active_call_code()),
            CallMicMode::PushToMute => self.push_to_mute,
        }
    }
}

impl<'de> Deserialize<'de> for TransmitConfig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize, Default)]
        enum TransmitMode {
            #[default]
            VoiceActivation,
            PushToTalk,
            PushToMute,
            RadioIntegration,
        }

        #[derive(Deserialize, Default)]
        struct TransmitConfigRaw {
            call_mic_mode: Option<CallMicMode>,
            mode: Option<TransmitMode>,
            push_to_talk: Option<Code>,
            push_to_mute: Option<Code>,
            radio_push_to_talk: Option<Code>,
        }

        let raw = TransmitConfigRaw::deserialize(deserializer)?;

        // Migrate old TransmitMode
        if let Some(mode) = raw.mode {
            let call_mic_mode = match mode {
                TransmitMode::VoiceActivation => CallMicMode::VoiceActivation,
                TransmitMode::PushToTalk | TransmitMode::RadioIntegration => {
                    CallMicMode::PushToTalk
                }
                TransmitMode::PushToMute => CallMicMode::PushToMute,
            };

            let is_radio_integration = matches!(mode, TransmitMode::RadioIntegration);

            let push_to_talk = if is_radio_integration {
                raw.radio_push_to_talk
            } else {
                raw.push_to_talk
            };

            return Ok(TransmitConfig {
                call_mic_mode,
                push_to_talk,
                push_to_mute: raw.push_to_mute,
                radio_push_to_talk: raw.radio_push_to_talk,
                was_radio_integration: Some(is_radio_integration),
            });
        }

        if let Some(call_mic_mode) = raw.call_mic_mode {
            return Ok(TransmitConfig {
                call_mic_mode,
                push_to_talk: raw.push_to_talk,
                push_to_mute: raw.push_to_mute,
                radio_push_to_talk: raw.radio_push_to_talk,
                was_radio_integration: None,
            });
        }

        Ok(TransmitConfig::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrontendTransmitConfig {
    pub call_mic_mode: CallMicMode,
    pub push_to_talk: Option<String>,
    pub push_to_mute: Option<String>,
    pub radio_push_to_talk: Option<String>,
}

impl From<TransmitConfig> for FrontendTransmitConfig {
    fn from(transmit_config: TransmitConfig) -> Self {
        Self {
            call_mic_mode: transmit_config.call_mic_mode,
            push_to_talk: transmit_config.push_to_talk.map(|c| c.to_string()),
            push_to_mute: transmit_config.push_to_mute.map(|c| c.to_string()),
            radio_push_to_talk: transmit_config.radio_push_to_talk.map(|c| c.to_string()),
        }
    }
}

impl TryFrom<FrontendTransmitConfig> for TransmitConfig {
    type Error = Error;

    fn try_from(value: FrontendTransmitConfig) -> Result<Self, Self::Error> {
        let push_to_talk = value
            .push_to_talk
            .as_ref()
            .map(|s| s.parse::<Code>())
            .transpose()
            .map_err(|_| Error::Other(Box::new(anyhow::anyhow!("Unrecognized key code: {}. Please report this error in our GitHub repository's issue tracker.", value.push_to_talk.unwrap_or_default()))))?;
        let push_to_mute = value
            .push_to_mute
            .as_ref()
            .map(|s| s.parse::<Code>())
            .transpose()
            .map_err(|_| Error::Other(Box::new(anyhow::anyhow!("Unrecognized key code: {}. Please report this error in our GitHub repository's issue tracker.", value.push_to_mute.unwrap_or_default()))))?;
        let radio_push_to_talk = value
            .radio_push_to_talk
            .as_ref()
            .map(|s| s.parse::<Code>())
            .transpose()
            .map_err(|_| Error::Other(Box::new(anyhow::anyhow!("Unrecognized key code: {}. Please report this error in our GitHub repository's issue tracker.", value.radio_push_to_talk.unwrap_or_default()))))?;

        Ok(Self {
            call_mic_mode: value.call_mic_mode,
            push_to_talk,
            push_to_mute,
            radio_push_to_talk,
            was_radio_integration: None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadioConfig {
    pub integration: Option<RadioIntegration>,
    pub audio_for_vatsim: Option<AudioForVatsimRadioConfig>,
    pub track_audio: Option<TrackAudioRadioConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioForVatsimRadioConfig {
    pub emit: Option<Code>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrackAudioRadioConfig {
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrontendRadioConfig {
    pub integration: Option<RadioIntegration>,
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
    pub endpoint: Option<String>,
}

impl RadioConfig {
    /// Create a radio integration instance based on the configured integration type.
    ///
    /// Returns `None` if the integration is not configured or if the emit key is not set.
    ///
    /// # Platform Limitation
    ///
    /// **Important**: AudioForVatsim Radio integration requires a functional `KeybindEmitter` to
    /// inject key presses into external applications. This works on Windows and macOS, but NOT
    /// on Linux where the emitter is a no-op stub due to Wayland's security model.
    ///
    /// On Linux, this method will successfully create a radio instance, but it will
    /// silently do nothing when `transmit()` is called.
    ///
    /// The TrackAudio integration is not affected by this platform limitation and is thus the
    /// default radio implementation for Linux.
    pub async fn radio(&self, app: AppHandle) -> Result<Option<DynRadio>, Error> {
        match self.integration {
            Some(RadioIntegration::AudioForVatsim) => {
                let Some(config) = self.audio_for_vatsim.as_ref() else {
                    return Ok(None);
                };
                let Some(emit) = config.emit else {
                    return Ok(None);
                };
                log::debug!("Initializing AudioForVatsim radio integration");
                let radio = PushToTalkRadio::new(app, emit).map_err(Error::from)?;
                Ok(Some(Arc::new(radio)))
            }
            Some(RadioIntegration::TrackAudio) => {
                let endpoint = self.track_audio.as_ref().and_then(|c| c.endpoint.as_ref());
                log::debug!("Initializing TrackAudio radio integration (endpoint: {endpoint:?})");
                let radio = Arc::new(
                    TrackAudioRadio::new(app.clone(), endpoint)
                        .await
                        .map_err(Error::from)?,
                );
                Ok(Some(radio))
            }
            _ => Ok(None),
        }
    }

    pub async fn validate(&self, transmit_config: &TransmitConfig) -> Result<(), Error> {
        if self.integration == Some(RadioIntegration::AudioForVatsim)
            && let Some(afv_code) = self.audio_for_vatsim.as_ref().and_then(|c| c.emit)
            && let Some(radio_code) = transmit_config.active_radio_code(true).await
            && afv_code == radio_code
        {
            return Err(KeybindsError::Other(
                "AFV emit key must be distinct from your radio integration push-to-talk key"
                    .to_string(),
            )
            .into());
        }
        Ok(())
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
            endpoint: value.endpoint,
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
            endpoint: value.endpoint,
        })
    }
}

/// Configuration for generic call control keybinds.
///
/// These keybinds allow accepting and ending calls as well as toggling radio prio without needing
/// to use the UI and can be used independently of the transmit mode.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeybindsConfig {
    /// Key code to accept an incoming call.
    pub accept_call: Option<Code>,
    /// Key code to end an active call.
    pub end_call: Option<Code>,
    /// Key code to toggle radio prio during an active call.
    pub toggle_radio_prio: Option<Code>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrontendKeybindsConfig {
    pub accept_call: Option<String>,
    pub end_call: Option<String>,
    pub toggle_radio_prio: Option<String>,
}

impl From<KeybindsConfig> for FrontendKeybindsConfig {
    fn from(config: KeybindsConfig) -> Self {
        Self {
            accept_call: config.accept_call.map(|c| c.to_string()),
            end_call: config.end_call.map(|c| c.to_string()),
            toggle_radio_prio: config.toggle_radio_prio.map(|c| c.to_string()),
        }
    }
}

impl TryFrom<FrontendKeybindsConfig> for KeybindsConfig {
    type Error = Error;

    fn try_from(value: FrontendKeybindsConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            accept_call: value
                .accept_call
                .as_ref()
                .map(|s| s.parse::<Code>())
                .transpose()
                .map_err(|_| Error::Other(Box::new(anyhow::anyhow!("Unrecognized key code: {}. Please report this error in our GitHub repository's issue tracker.", value.accept_call.unwrap_or_default()))))?,
            end_call: value
                .end_call
                .as_ref()
                .map(|s| s.parse::<Code>())
                .transpose()
                .map_err(|_| Error::Other(Box::new(anyhow::anyhow!("Unrecognized key code: {}. Please report this error in our GitHub repository's issue tracker.", value.end_call.unwrap_or_default()))))?,
            toggle_radio_prio: value
                .toggle_radio_prio
                .as_ref()
                .map(|s| s.parse::<Code>())
                .transpose()
                .map_err(|_| Error::Other(Box::new(anyhow::anyhow!("Unrecognized key code: {}. Please report this error in our GitHub repository's issue tracker.", value.toggle_radio_prio.unwrap_or_default()))))?,
        })
    }
}

/// Various settings regarding calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallConfig {
    /// Toggles highlighting of incoming call target DA keys.
    #[serde(default = "default_true")]
    pub highlight_incoming_call_target: bool,
    /// Enables the priority call ringtone and visual highlighting. If disabled, Priority calls will still be received, but not handled differently.
    #[serde(default = "default_true")]
    pub enable_priority_calls: bool,
    /// Enables sound effect when a call is established
    #[serde(default = "default_true")]
    pub enable_call_start_sound: bool,
    /// Enables sound effect when the call is ended
    #[serde(default = "default_true")]
    pub enable_call_end_sound: bool,
    /// Enables default call source selection based on the dataset position
    #[serde(default = "default_true")]
    pub use_default_call_sources: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendCallConfig {
    pub highlight_incoming_call_target: bool,
    pub enable_priority_calls: bool,
    pub enable_call_start_sound: bool,
    pub enable_call_end_sound: bool,
    pub use_default_call_sources: bool,
}

impl Default for CallConfig {
    fn default() -> Self {
        Self {
            highlight_incoming_call_target: true,
            enable_priority_calls: true,
            enable_call_start_sound: true,
            enable_call_end_sound: true,
            use_default_call_sources: true,
        }
    }
}

impl Default for FrontendCallConfig {
    fn default() -> Self {
        Self {
            highlight_incoming_call_target: true,
            enable_priority_calls: true,
            enable_call_start_sound: true,
            enable_call_end_sound: true,
            use_default_call_sources: true,
        }
    }
}

impl From<CallConfig> for FrontendCallConfig {
    fn from(call_config: CallConfig) -> Self {
        Self {
            highlight_incoming_call_target: call_config.highlight_incoming_call_target,
            enable_priority_calls: call_config.enable_priority_calls,
            enable_call_start_sound: call_config.enable_call_start_sound,
            enable_call_end_sound: call_config.enable_call_end_sound,
            use_default_call_sources: call_config.use_default_call_sources,
        }
    }
}

impl From<FrontendCallConfig> for CallConfig {
    fn from(frontend_call_config: FrontendCallConfig) -> Self {
        Self {
            highlight_incoming_call_target: frontend_call_config.highlight_incoming_call_target,
            enable_priority_calls: frontend_call_config.enable_priority_calls,
            enable_call_start_sound: frontend_call_config.enable_call_start_sound,
            enable_call_end_sound: frontend_call_config.enable_call_end_sound,
            use_default_call_sources: frontend_call_config.use_default_call_sources,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientPageSettings {
    /// Named configs for different client page configurations.
    /// Users can switch between configs in the UI.
    #[serde(default)]
    pub configs: HashMap<String, ClientPageConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendClientPageSettings {
    selected: Option<String>,
    configs: HashMap<String, FrontendClientPageConfig>,
}

impl From<&AppConfig> for FrontendClientPageSettings {
    fn from(config: &AppConfig) -> Self {
        FrontendClientPageSettings {
            selected: config.client.selected_client_page_config.clone(),
            configs: config
                .client_page
                .configs
                .iter()
                .map(|(k, v)| (k.clone(), v.clone().into()))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendClientPageConfig {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub priority: Vec<String>,
    pub frequencies: FrequencyDisplayMode,
    pub grouping: ClientGroupMode,
}

impl Default for FrontendClientPageConfig {
    fn default() -> Self {
        Self::from(ClientPageConfig::default())
    }
}

impl From<ClientPageConfig> for FrontendClientPageConfig {
    fn from(client_page_config: ClientPageConfig) -> Self {
        Self {
            include: client_page_config.include,
            exclude: client_page_config.exclude,
            priority: client_page_config.priority,
            frequencies: client_page_config.frequencies,
            grouping: client_page_config.grouping,
        }
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
