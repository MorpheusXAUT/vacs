mod listener;
pub use listener::*;

use ashpd::desktop::global_shortcuts::NewShortcut;
use keyboard_types::Code;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortalShortcutId {
    PushToTalk,
    PushToMute,
    RadioIntegration,
}

impl PortalShortcutId {
    pub const fn as_str(&self) -> &str {
        match self {
            PortalShortcutId::PushToTalk => "push_to_talk",
            PortalShortcutId::PushToMute => "push_to_mute",
            PortalShortcutId::RadioIntegration => "radio_integration",
        }
    }

    pub const fn description(&self) -> &'static str {
        match self {
            PortalShortcutId::PushToTalk => "Push-to-talk (activate voice transmission while held)",
            PortalShortcutId::PushToMute => "Push-to-mute (mute microphone while held)",
            PortalShortcutId::RadioIntegration => "Radio Integration",
        }
    }

    pub const fn all() -> &'static [Self] {
        &[
            PortalShortcutId::PushToTalk,
            PortalShortcutId::PushToMute,
            PortalShortcutId::RadioIntegration,
        ]
    }

    pub const fn from_transmit_mode(mode: crate::config::TransmitMode) -> Option<Self> {
        match mode {
            crate::config::TransmitMode::PushToTalk => Some(PortalShortcutId::PushToTalk),
            crate::config::TransmitMode::PushToMute => Some(PortalShortcutId::PushToMute),
            crate::config::TransmitMode::RadioIntegration => {
                Some(PortalShortcutId::RadioIntegration)
            }
            _ => None,
        }
    }
}

impl FromStr for PortalShortcutId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "push_to_talk" => Ok(PortalShortcutId::PushToTalk),
            "push_to_mute" => Ok(PortalShortcutId::PushToMute),
            "radio_integration" => Ok(PortalShortcutId::RadioIntegration),
            _ => Err(format!("unknown portal shortcut id {s}")),
        }
    }
}

impl TryFrom<&str> for PortalShortcutId {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<String> for PortalShortcutId {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.as_str().parse()
    }
}

impl AsRef<str> for PortalShortcutId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&PortalShortcutId> for NewShortcut {
    fn from(value: &PortalShortcutId) -> Self {
        NewShortcut::new(value.as_str(), value.description())
    }
}

impl From<PortalShortcutId> for NewShortcut {
    fn from(value: PortalShortcutId) -> Self {
        NewShortcut::new(value.as_str(), value.description())
    }
}

impl From<PortalShortcutId> for Code {
    fn from(value: PortalShortcutId) -> Self {
        match value {
            PortalShortcutId::PushToTalk => Code::F33,
            PortalShortcutId::PushToMute => Code::F34,
            PortalShortcutId::RadioIntegration => Code::F35,
        }
    }
}

impl TryFrom<Code> for PortalShortcutId {
    type Error = String;
    fn try_from(value: Code) -> Result<Self, Self::Error> {
        match value {
            Code::F33 => Ok(PortalShortcutId::PushToTalk),
            Code::F34 => Ok(PortalShortcutId::PushToMute),
            Code::F35 => Ok(PortalShortcutId::RadioIntegration),
            _ => Err(format!("unknown portal shortcut code {value}")),
        }
    }
}
