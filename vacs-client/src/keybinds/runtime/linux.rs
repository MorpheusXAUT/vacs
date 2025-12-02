mod wayland;

use crate::keybinds::runtime::{KeybindEmitter, KeybindListener, stub};
use crate::keybinds::{KeyEvent, KeybindsError};
use crate::platform::Platform;
use keyboard_types::{Code, KeyState};
use std::fmt::{Debug, Formatter};
use tokio::sync::mpsc::UnboundedReceiver;

pub enum LinuxKeybindListener {
    Wayland(wayland::WaylandKeybindListener),
    X11(stub::NoopKeybindListener),
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
    async fn start() -> Result<(Self, UnboundedReceiver<KeyEvent>), KeybindsError>
    where
        Self: Sized,
    {
        match Platform::detect() {
            Platform::LinuxWayland => {
                let (listener, rx) = wayland::WaylandKeybindListener::start().await?;
                Ok((Self::Wayland(listener), rx))
            }
            Platform::LinuxX11 => {
                let (listener, rx) = stub::NoopKeybindListener::start().await?;
                Ok((Self::X11(listener), rx))
            }
            Platform::LinuxUnknown => {
                let (listener, rx) = stub::NoopKeybindListener::start().await?;
                Ok((Self::Stub(listener), rx))
            }
            platform => Err(KeybindsError::Listener(format!(
                "Unsupported platform {platform} for LinuxKeybindListener",
            ))),
        }
    }

    fn get_external_binding(&self, mode: crate::config::TransmitMode) -> Option<String> {
        match self {
            Self::Wayland(l) => l.get_external_binding(mode),
            Self::X11(_) => None,
            Self::Stub(_) => None,
        }
    }
}

pub enum LinuxKeybindEmitter {
    Wayland(stub::NoopKeybindEmitter),
    X11(stub::NoopKeybindEmitter),
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
        match Platform::detect() {
            Platform::LinuxWayland => Ok(Self::Wayland(stub::NoopKeybindEmitter::start()?)),
            Platform::LinuxX11 => Ok(Self::X11(stub::NoopKeybindEmitter::start()?)),
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
