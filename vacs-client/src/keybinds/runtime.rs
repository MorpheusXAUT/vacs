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
//! | Linux X11      | ⏳       | ⏳      | Stub implementations (to be implemented) |
//!
//! On Wayland, we can listen to global shortcuts via the XDG Desktop Portal, but there's
//! no standard API for injecting input events (security model). This means radio integration
//! (which requires emitting key presses) doesn't work on Wayland.
//!
//! ## Platform Selection
//!
//! Platform-specific implementations are selected at compile time using `cfg_if!`:
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

use crate::keybinds::{KeyEvent, KeybindsError};
use keyboard_types::{Code, KeyState};
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

/// Trait for platform-specific keybind listeners that capture global keyboard events.
///
/// Implementations must be thread-safe (`Send + Sync`) and start asynchronously to allow
/// for platform-specific initialization (e.g., connecting to XDG portal on Wayland).
pub trait KeybindListener: Send + Sync + Debug + 'static {
    async fn start() -> Result<(Self, UnboundedReceiver<KeyEvent>), KeybindsError>
    where
        Self: Sized;

    /// Get the external (OS-configured) keybind for a transmit mode, if available.
    ///
    /// This is used on platforms where keybinds are configured at the OS level
    /// (e.g., Wayland via XDG Global Shortcuts portal) rather than in the application.
    ///
    /// Returns `None` by default for platforms where keybinds are app-configured.
    #[allow(dead_code)]
    fn get_external_binding(&self, _mode: crate::config::TransmitMode) -> Option<String> {
        None
    }
}

pub type DynKeybindListener = Arc<dyn KeybindListener>;

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

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        mod windows;
        pub use windows::WindowsKeybindEmitter as PlatformEmitter;
        pub use windows::WindowsKeybindListener as PlatformListener;
    } else if #[cfg(target_os = "macos")] {
        mod macos;
        pub use macos::MacOsKeybindListener as PlatformListener;
        pub use macos::MacOsKeybindEmitter as PlatformEmitter;
    } else if #[cfg(target_os = "linux")] {
        mod linux;
        mod stub;
        pub use linux::LinuxKeybindEmitter as PlatformEmitter;
        pub use linux::LinuxKeybindListener as PlatformListener;
    } else {
        mod stub;
        pub use stub::NoopKeybindEmitter as PlatformEmitter;
        pub use stub::NoopKeybindListener as PlatformListener;
    }
}
