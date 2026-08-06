//! Wayland keybind listener implementation using XDG Global Shortcuts portal.
//!
//! # Overview
//!
//! This module implements global keybind listening for Wayland compositors using the
//! [XDG Desktop Portal](https://flatpak.github.io/xdg-desktop-portal/) Global Shortcuts API.
//!
//! ## Why XDG Portal?
//!
//! Unlike X11 where applications can directly listen to global keyboard events, Wayland's
//! security model requires applications to request permission from the compositor. The
//! XDG Desktop Portal provides a standardized D-Bus API for this purpose.
//!
//! ## Compositor Support
//!
//! This implementation works on compositors that support the Global Shortcuts portal:
//! - KDE Plasma (via `xdg-desktop-portal-kde`)
//! - GNOME (via `xdg-desktop-portal-gnome`)
//! - Hyprland (via `xdg-desktop-portal-hyprland`)
//!
//! ## Semantic Events
//!
//! The portal allows complex key combinations (e.g., `Ctrl+Alt+Shift+P`) that cannot be
//! represented as a single `keyboard_types::Code`. Portal activations are therefore
//! forwarded as semantic [`Trigger::Portal`](crate::keybinds::Trigger) events carrying
//! the activated [`PortalAction`], which the keybind engine matches directly against
//! the active trigger set - no physical key identity is involved.
//!
//! ## User Experience
//!
//! 1. On first launch, the compositor shows a configuration dialog
//! 2. User configures their preferred key combinations
//! 3. Shortcuts are stored by the compositor and persist across app restarts
//! 4. User can reconfigure shortcuts in their desktop environment settings

mod listener;

pub use listener::*;
use std::collections::HashMap;

use crate::keybinds::{Keybind, PortalAction};
use ashpd::desktop::global_shortcuts::NewShortcut;
use parking_lot::RwLock;
use std::str::FromStr;
use std::sync::Arc;

/// Identifiers for shortcuts registered with the XDG Global Shortcuts portal.
///
/// Each variant corresponds to a transmit mode in vacs. These IDs are used to:
/// - Register shortcuts with the portal
/// - Identify which shortcut was activated in portal signals
/// - Query the current key binding from the portal

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortalShortcutId {
    PushToTalk,
    PushToMute,
    RadioPushToTalk,
    CallControl,
    ToggleRadioPrio,
}

impl PortalShortcutId {
    pub const fn as_str(&self) -> &'static str {
        match self {
            PortalShortcutId::PushToTalk => "push_to_talk",
            PortalShortcutId::PushToMute => "push_to_mute",
            PortalShortcutId::RadioPushToTalk => "radio_push_to_talk",
            PortalShortcutId::CallControl => "call_control",
            PortalShortcutId::ToggleRadioPrio => "toggle_radio_prio",
        }
    }

    pub const fn description(&self) -> &'static str {
        match self {
            PortalShortcutId::PushToTalk => "Push-to-talk (activate voice transmission while held)",
            PortalShortcutId::PushToMute => "Push-to-mute (mute microphone while held)",
            PortalShortcutId::RadioPushToTalk => {
                "Radio Push-to-talk (activate radio transmission while held)"
            }
            PortalShortcutId::CallControl => "Call Control (end active/accept next)",
            PortalShortcutId::ToggleRadioPrio => "Toggle Radio Priority (during active call)",
        }
    }

    pub const fn all() -> &'static [Self] {
        &[
            PortalShortcutId::PushToTalk,
            PortalShortcutId::PushToMute,
            PortalShortcutId::RadioPushToTalk,
            PortalShortcutId::CallControl,
            PortalShortcutId::ToggleRadioPrio,
        ]
    }
}

impl FromStr for PortalShortcutId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "push_to_talk" => Ok(PortalShortcutId::PushToTalk),
            "push_to_mute" => Ok(PortalShortcutId::PushToMute),
            "radio_push_to_talk" => Ok(PortalShortcutId::RadioPushToTalk),
            "call_control" => Ok(PortalShortcutId::CallControl),
            "toggle_radio_prio" => Ok(PortalShortcutId::ToggleRadioPrio),
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

impl From<PortalShortcutId> for PortalAction {
    fn from(value: PortalShortcutId) -> Self {
        match value {
            PortalShortcutId::PushToTalk => PortalAction::PushToTalk,
            PortalShortcutId::PushToMute => PortalAction::PushToMute,
            PortalShortcutId::RadioPushToTalk => PortalAction::RadioPushToTalk,
            PortalShortcutId::CallControl => PortalAction::CallControl,
            PortalShortcutId::ToggleRadioPrio => PortalAction::ToggleRadioPrio,
        }
    }
}

impl From<Keybind> for PortalShortcutId {
    fn from(value: Keybind) -> Self {
        match value {
            Keybind::PushToTalk => PortalShortcutId::PushToTalk,
            Keybind::PushToMute => PortalShortcutId::PushToMute,
            Keybind::RadioPushToTalk => PortalShortcutId::RadioPushToTalk,
            Keybind::AcceptCall => PortalShortcutId::CallControl,
            Keybind::EndCall => PortalShortcutId::CallControl,
            Keybind::ToggleRadioPrio => PortalShortcutId::ToggleRadioPrio,
        }
    }
}

pub async fn is_portal_shortcut_bound(shortcut_id: PortalShortcutId) -> bool {
    let (proxy, session) = match initialize_portal(&mut None).await {
        Ok(res) => res,
        Err(err) => {
            log::error!("Failed to initialize Wayland Global Shortcuts portal: {err}");
            return false;
        }
    };

    let shortcuts = Arc::new(RwLock::new(HashMap::new()));

    let bound = match check_existing_shortcuts(&proxy, &session, &mut None, &shortcuts).await {
        Ok(()) => shortcuts.read().contains_key(&shortcut_id),
        Err(err) => {
            log::error!("Failed to check existing Wayland Global Shortcuts: {err}");
            false
        }
    };

    // This session only exists for the duration of the query. It must be closed
    // explicitly: the portal keeps leaked sessions alive and re-emits every
    // shortcut signal once per session, so each leak would deliver an extra
    // duplicate of all future Activated/Deactivated/ShortcutsChanged signals to
    // the real listener.
    match tokio::time::timeout(std::time::Duration::from_secs(2), session.close()).await {
        Ok(Ok(())) => {}
        Ok(Err(err)) => log::warn!("Failed to close global shortcuts query session: {err}"),
        Err(_) => log::warn!("Timed out closing global shortcuts query session"),
    }

    bound
}
