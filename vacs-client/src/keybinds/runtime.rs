use crate::keybinds::{KeyEvent, KeybindsError};
use tokio::sync::mpsc::UnboundedReceiver;

pub trait KeybindRuntime: Send + 'static {
    fn start() -> Result<(Self, UnboundedReceiver<KeyEvent>), KeybindsError>
    where
        Self: Sized;

    fn stop(&mut self);
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::WindowsKeybindRuntime as PlatformKeybindRuntime;

#[cfg(not(any(target_os = "windows")))]
mod stub;
#[cfg(not(any(target_os = "windows")))]
pub use stub::NoopKeybindRuntime as PlatformKeybindRuntime;
