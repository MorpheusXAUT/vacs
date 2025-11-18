use serde::Serialize;
use std::fmt::Display;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Capabilities {
    pub always_on_top: bool,
    pub window_state: bool,
    pub keybinds: bool,

    pub platform: Platform,
}

impl Default for Capabilities {
    fn default() -> Self {
        let platform = Platform::detect();

        Self {
            always_on_top: !matches!(platform, Platform::LinuxWayland),
            window_state: !matches!(platform, Platform::LinuxWayland),
            keybinds: matches!(platform, Platform::Windows | Platform::MacOs),
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

impl Platform {
    pub fn detect() -> Platform {
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
