//! Platform-specific keybind listener and emitter implementations.
//!
//! # Architecture
//!
//! This module provides a trait-based abstraction for capturing and emitting keyboard events
//! across different platforms. The design separates two distinct capabilities:
//!
//! - **`KeybindListener`**: Captures global keyboard events (listening/input)
//! - **`KeybindEmitter`**: Injects keyboard events into other applications (emission/output)
//!
//! ## Why Separate Traits?
//!
//! Listener and emitter are separate traits because they have different platform support:
//!
//! | Platform       | Listener | Emitter | Notes                                    |
//! |----------------|----------|---------|------------------------------------------|
//! | Windows        | ✅       | ✅      | Full support via Win32 API               |
//! | macOS          | ✅       | ✅      | Full support via Accessibility API       |
//! | Linux Wayland  | ✅       | ❌      | Listener via XDG portal, no emitter API  |
//! | Linux X11      | ✅       | ✅      | XInput2 raw events / XTest injection     |
//!
//! On Wayland, we can listen to global shortcuts via the XDG Desktop Portal, but there's
//! no standard API for injecting input events (security model). This means radio integration
//! (which requires emitting key presses) doesn't work on Wayland.
//!
//! ## Platform Selection
//!
//! Platform-specific implementations are selected at compile time using `cfg_select!`:
//!
//! - **Windows/macOS**: Direct platform implementation
//! - **Linux**: Runtime detection (see `linux.rs`) to choose between Wayland/X11/Unknown
//! - **Other**: Stub no-op implementation
//!
//! ## External Bindings
//!
//! The `get_external_binding()` method on `KeybindListener` allows querying OS-configured
//! keybinds. This is currently only implemented for Wayland, where shortcuts are configured
//! in the desktop environment rather than in the app.

use crate::keybinds::{KeyEvent, Keybind, KeybindsError};
use keyboard_types::{Code, KeyState};
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;

/// Await the startup confirmation an input-source thread sends through a
/// oneshot channel, mapping the failure, channel-closed, and timeout cases to
/// uniform errors. Shared by all platform listeners and the joystick service.
pub(crate) async fn await_startup<T>(
    startup_rx: oneshot::Receiver<Result<T, KeybindsError>>,
    timeout: Duration,
    component: &str,
) -> Result<T, KeybindsError> {
    match tokio::time::timeout(timeout, startup_rx).await {
        Ok(Ok(Ok(value))) => {
            log::debug!("{component} started successfully");
            Ok(value)
        }
        Ok(Ok(Err(err))) => {
            log::error!("{component} startup failed: {err}");
            Err(err)
        }
        Ok(Err(_)) => {
            log::error!("{component} startup channel closed unexpectedly");
            Err(KeybindsError::Listener(format!(
                "{component} startup channel closed"
            )))
        }
        Err(_) => {
            log::error!("{component} startup timed out");
            Err(KeybindsError::Listener(format!(
                "{component} startup timed out"
            )))
        }
    }
}

/// Trait for platform-specific keybind listeners that capture global keyboard events.
///
/// Implementations must be thread-safe (`Send + Sync`) and start asynchronously to allow
/// for platform-specific initialization (e.g., connecting to XDG portal on Wayland).
///
/// Listeners send captured events into the channel provided by the caller; the
/// keybind engine hands the same channel to all active input sources (keyboard
/// listener, joystick service) so its event loop reads a single merged stream.
pub trait KeybindListener: Send + Sync + Debug + 'static {
    async fn start(key_event_tx: UnboundedSender<KeyEvent>) -> Result<Self, KeybindsError>
    where
        Self: Sized;

    /// Get the external (OS-configured) key for a keybind, if available.
    ///
    /// This is used on platforms where keybinds are configured at the OS level
    /// (e.g., Wayland via XDG Global Shortcuts portal) rather than in the application.
    ///
    /// Returns `None` by default for platforms where keybinds are app-configured.
    #[allow(dead_code)]
    fn get_external_binding(&self, _keybind: Keybind) -> Option<String> {
        None
    }

    /// Whether an OS level binding exists for a keybind.
    ///
    /// Separate from [`Self::get_external_binding`] because the keybind engine
    /// asks this on every key press and has no use for the binding string, which
    /// that method has to clone out of the listener's map.
    fn has_external_binding(&self, keybind: Keybind) -> bool {
        self.get_external_binding(keybind).is_some()
    }
}

pub type DynKeybindListener = Arc<dyn KeybindListener>;

/// Non-owning handle to a [`DynKeybindListener`], for holders that must not keep
/// a replaced listener (and its OS level session) alive.
pub type WeakKeybindListener = std::sync::Weak<dyn KeybindListener>;

/// Trait for platform-specific keybind emitters that inject keyboard events.
///
/// Implementations must be thread-safe (`Send + Sync`) and provide synchronous emission
/// since the actual key injection is typically a fast system call.
pub trait KeybindEmitter: Send + Sync + Debug + 'static {
    fn start() -> Result<Self, KeybindsError>
    where
        Self: Sized;

    fn emit(&self, code: Code, state: KeyState) -> Result<(), KeybindsError>;
}

pub type DynKeybindEmitter = Arc<dyn KeybindEmitter>;

cfg_select! {
    target_os = "windows" => {
        mod windows;
        pub use windows::WindowsKeybindEmitter as PlatformEmitter;
        pub use windows::WindowsKeybindListener as PlatformListener;
    }
    target_os = "macos" => {
        mod macos;
        pub use macos::MacOsKeybindListener as PlatformListener;
        pub use macos::MacOsKeybindEmitter as PlatformEmitter;
    }
    target_os = "linux" => {
        mod linux;
        mod stub;
        pub use linux::LinuxKeybindEmitter as PlatformEmitter;
        pub use linux::LinuxKeybindListener as PlatformListener;
    }
    _ => {
        mod stub;
        pub use stub::NoopKeybindEmitter as PlatformEmitter;
        pub use stub::NoopKeybindListener as PlatformListener;
    }
}
