//! Linux keybind listener and emitter implementations with runtime platform detection.
//!
//! # Architecture
//!
//! Linux has multiple display server protocols (X11, Wayland) with different capabilities:
//!
//! - **Wayland**: Uses XDG Global Shortcuts portal for listening. Emitter is not supported
//!   due to Wayland's security model (no global input injection).
//! - **X11**: Listens via XInput2 raw events (focus-independent, non-grabbing).
//! - **Unknown**: No display server detected, uses stub implementations.
//!
//! # Runtime Platform Detection
//!
//! Unlike Windows and macOS where the platform is known at compile time, Linux requires
//! runtime detection to determine whether we're running on X11 or Wayland. This is done
//! by checking environment variables (XDG_SESSION_TYPE, WAYLAND_DISPLAY, DISPLAY).
//!
//! The `LinuxKeybindListener` and `LinuxKeybindEmitter` enums wrap the platform-specific
//! implementations and dispatch to the correct one based on runtime detection.
//!
//! # Emitter Limitation
//!
//! **Important**: The emitter is only functional on X11 (via the XTest extension). On
//! Wayland it remains a no-op stub — there is no standard API for global input injection
//! (security model) — so radio integration via emitted key presses does not work there.

mod wayland;
mod x11;

use crate::keybinds::runtime::{KeybindEmitter, KeybindListener, stub};
use crate::keybinds::{KeyEvent, Keybind, KeybindsError};
use crate::platform::Platform;
use keyboard_types::{Code, KeyState};
use std::fmt::{Debug, Formatter};
use tokio::sync::mpsc::UnboundedSender;

pub enum LinuxKeybindListener {
    Wayland(wayland::WaylandKeybindListener),
    X11(x11::X11KeybindListener),
    Stub(stub::NoopKeybindListener),
}

impl Debug for LinuxKeybindListener {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wayland(l) => write!(f, "LinuxKeybindListener::Wayland({l:?})"),
            Self::X11(l) => write!(f, "LinuxKeybindListener::X11({l:?})"),
            Self::Stub(l) => write!(f, "LinuxKeybindListener::Stub({l:?})"),
        }
    }
}

impl KeybindListener for LinuxKeybindListener {
    async fn start(key_event_tx: UnboundedSender<KeyEvent>) -> Result<Self, KeybindsError>
    where
        Self: Sized,
    {
        // Runtime platform detection to select the appropriate listener implementation
        match Platform::get() {
            Platform::LinuxWayland => Ok(Self::Wayland(
                wayland::WaylandKeybindListener::start(key_event_tx).await?,
            )),
            Platform::LinuxX11 => Ok(Self::X11(
                x11::X11KeybindListener::start(key_event_tx).await?,
            )),
            Platform::LinuxUnknown => Ok(Self::Stub(
                stub::NoopKeybindListener::start(key_event_tx).await?,
            )),
            platform => Err(KeybindsError::Listener(format!(
                "Unsupported platform {platform} for LinuxKeybindListener",
            ))),
        }
    }

    fn get_external_binding(&self, keybind: Keybind) -> Option<String> {
        match self {
            Self::Wayland(l) => l.get_external_binding(keybind),
            Self::X11(_) => None,
            Self::Stub(_) => None,
        }
    }

    fn has_external_binding(&self, keybind: Keybind) -> bool {
        match self {
            Self::Wayland(l) => l.has_external_binding(keybind),
            Self::X11(_) => false,
            Self::Stub(_) => false,
        }
    }
}

pub enum LinuxKeybindEmitter {
    Wayland(stub::NoopKeybindEmitter),
    // Boxed: the X connection makes this variant much larger than the stubs
    X11(Box<x11::X11KeybindEmitter>),
    Stub(stub::NoopKeybindEmitter),
}

impl Debug for LinuxKeybindEmitter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wayland(l) => write!(f, "LinuxKeybindEmitter::Wayland({l:?})"),
            Self::X11(l) => write!(f, "LinuxKeybindEmitter::X11({l:?})"),
            Self::Stub(l) => write!(f, "LinuxKeybindEmitter::Stub({l:?})"),
        }
    }
}

impl KeybindEmitter for LinuxKeybindEmitter {
    fn start() -> Result<Self, KeybindsError>
    where
        Self: Sized,
    {
        // Runtime platform detection to select the appropriate emitter implementation.
        //
        // NOTE: Wayland and Unknown use the stub implementation:
        // - Wayland: No standard API for global input injection (security model)
        // - Unknown: No display server available
        match Platform::get() {
            Platform::LinuxWayland => Ok(Self::Wayland(stub::NoopKeybindEmitter::start()?)),
            Platform::LinuxX11 => Ok(Self::X11(Box::new(x11::X11KeybindEmitter::start()?))),
            Platform::LinuxUnknown => Ok(Self::Stub(stub::NoopKeybindEmitter::start()?)),
            platform => Err(KeybindsError::Emitter(format!(
                "Unsupported platform {platform} for LinuxKeybindEmitter",
            ))),
        }
    }

    fn emit(&self, code: Code, state: KeyState) -> Result<(), KeybindsError> {
        match self {
            Self::Wayland(emitter) => emitter.emit(code, state),
            Self::X11(emitter) => emitter.emit(code, state),
            Self::Stub(emitter) => emitter.emit(code, state),
        }
    }
}
