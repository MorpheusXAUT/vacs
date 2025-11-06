use crate::keybinds::{KeyEvent, KeybindsError};
use keyboard_types::{Code, KeyState};
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

pub trait KeybindListener: Send + Sync + Debug + 'static {
    fn start() -> Result<(Self, UnboundedReceiver<KeyEvent>), KeybindsError>
    where
        Self: Sized;
}

pub type DynKeybindListener = Arc<dyn KeybindListener>;

pub trait KeybindEmitter: Send + Sync + Debug + 'static {
    fn start() -> Result<Self, KeybindsError>
    where
        Self: Sized;

    fn emit(&self, code: Code, state: KeyState) -> Result<(), KeybindsError>;
}

pub type DynKeybindEmitter = Arc<dyn KeybindEmitter>;

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        mod windows;
        pub use windows::WindowsKeybindEmitter as PlatformEmitter;
        pub use windows::WindowsKeybindListener as PlatformListener;
    } else if #[cfg(target_os = "macos")] {
        mod macos;
        pub use macos::MacOsKeybindListener as PlatformListener;
        mod stub;
        pub use stub::NoopKeybindEmitter as PlatformEmitter;
    } else {
        mod stub;
        pub use stub::NoopKeybindEmitter as PlatformEmitter;
        pub use stub::NoopKeybindListener as PlatformListener;
    }
}
