use crate::config::frontend_config;
use crate::error::Error as VacsError;
use keyboard_types::{Code, KeyState};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

pub mod commands;
pub mod engine;
pub mod runtime;

#[derive(Debug, Clone, Error)]
pub enum KeybindsError {
    #[error("Keybinds listener error: {0}")]
    Listener(String),
    #[error("Keybinds emitter error: {0}")]
    Emitter(String),
    #[error("Unrecognized keybinds code: {0}")]
    UnrecognizedCode(String),
    #[error("Fake marker")]
    FakeMarker,
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone)]
pub struct KeyEvent {
    code: Code,
    #[allow(dead_code)]
    label: String,
    state: KeyState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Keybind {
    PushToTalk,
    PushToMute,
    RadioPushToTalk,
    AcceptCall,
    EndCall,
    ToggleRadioPrio,
}

/// Parse an optional frontend key-code string (e.g. `"KeyA"`) into a [`Code`].
///
/// Returns a user-facing error if the string is not a recognized key code.
pub(crate) fn parse_key_code(code: Option<String>) -> Result<Option<Code>, VacsError> {
    code.map(|s| {
        s.parse::<Code>().map_err(|_| {
            VacsError::Other(Box::new(anyhow::anyhow!(
                "Unrecognized key code: {s}. Please report this error in our GitHub repository's issue tracker."
            )))
        })
    })
    .transpose()
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
    type Error = VacsError;

    fn try_from(value: FrontendTransmitConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            call_mic_mode: value.call_mic_mode,
            push_to_talk: parse_key_code(value.push_to_talk)?,
            push_to_mute: parse_key_code(value.push_to_mute)?,
            radio_push_to_talk: parse_key_code(value.radio_push_to_talk)?,
            was_radio_integration: None,
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

frontend_config!(KeybindsConfig => FrontendKeybindsConfig {
    key accept_call,
    key end_call,
    key toggle_radio_prio,
});
