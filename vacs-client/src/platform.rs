use serde::Serialize;
use std::fmt::Display;
use std::sync::OnceLock;

/// Platform capabilities that determine which features are available.
///
/// Different platforms have different capabilities due to OS-level restrictions:
/// - **Windows/macOS**: Full keybind listener and emitter support
/// - **Linux Wayland**: Listener support via XDG portal, but no emitter (security model)
/// - **Linux X11**: Currently stub implementations (to be implemented)
/// - **Linux Unknown**: No display server detected, stub implementations
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Capabilities {
    pub always_on_top: bool,
    pub keybind_listener: bool,
    pub keybind_emitter: bool,

    pub platform: Platform,
}

impl Default for Capabilities {
    fn default() -> Self {
        let platform = Platform::get();

        Self {
            always_on_top: !matches!(platform, Platform::LinuxWayland),
            keybind_listener: matches!(
                platform,
                Platform::Windows | Platform::MacOs | Platform::LinuxWayland
            ),
            keybind_emitter: matches!(platform, Platform::Windows | Platform::MacOs),
            platform,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[allow(dead_code)]
pub enum Platform {
    Unknown,
    Windows,
    MacOs,
    LinuxX11,
    LinuxWayland,
    LinuxUnknown,
}

/// Cached platform detection result.
///
/// Platform detection reads environment variables, which is relatively expensive.
/// Since the platform cannot change during application runtime (you can't switch
/// from X11 to Wayland without restarting the app), we cache the result using
/// `OnceLock` for thread-safe lazy initialization.
static PLATFORM_CACHE: OnceLock<Platform> = OnceLock::new();

impl Platform {
    /// Get the current platform, using a cached value if available.
    ///
    /// This is the preferred method for platform detection as it avoids repeated
    /// environment variable lookups. The first call will perform detection and
    /// cache the result; subsequent calls return the cached value.
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called from multiple threads
    /// simultaneously. Only one thread will perform the actual detection.
    pub fn get() -> Platform {
        *PLATFORM_CACHE.get_or_init(Self::detect)
    }

    /// Detect the current platform by examining environment variables.
    ///
    /// This method performs the actual platform detection logic. It should not
    /// be called directly in most cases; use `Platform::get()` instead to benefit
    /// from caching.
    ///
    /// # Detection Strategy
    ///
    /// On Linux, we check in order:
    /// 1. `XDG_SESSION_TYPE` environment variable (most reliable)
    /// 2. `WAYLAND_DISPLAY` environment variable (fallback for Wayland)
    /// 3. `DISPLAY` environment variable (fallback for X11)
    /// 4. If none are set, return `LinuxUnknown`
    fn detect() -> Platform {
        #[cfg(target_os = "windows")]
        {
            Platform::Windows
        }

        #[cfg(target_os = "macos")]
        {
            Platform::MacOs
        }

        #[cfg(target_os = "linux")]
        {
            use std::env;
            if let Ok(xdg_session_type) = env::var("XDG_SESSION_TYPE") {
                match xdg_session_type.to_lowercase().as_str() {
                    "wayland" => return Platform::LinuxWayland,
                    "x11" => return Platform::LinuxX11,
                    _ => {}
                }
            }

            if env::var("WAYLAND_DISPLAY").is_ok() {
                Platform::LinuxWayland
            } else if env::var("DISPLAY").is_ok() {
                Platform::LinuxX11
            } else {
                Platform::LinuxUnknown
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Platform::Unknown
        }
    }

    #[allow(dead_code)]
    pub fn is_linux(&self) -> bool {
        matches!(
            self,
            Platform::LinuxX11 | Platform::LinuxWayland | Platform::LinuxUnknown
        )
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Windows => "Windows",
            Platform::MacOs => "MacOs",
            Platform::LinuxX11 => "LinuxX11",
            Platform::LinuxWayland => "LinuxWayland",
            Platform::LinuxUnknown => "LinuxUnknown",
            Platform::Unknown => "Unknown",
        }
    }
}

impl Default for Platform {
    fn default() -> Self {
        Self::detect()
    }
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
