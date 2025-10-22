use crate::keybinds::KeybindsError;
use keyboard_types::Code;
use std::fmt::{Debug, Formatter};
use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;

mod emitter;
pub use emitter::*;
mod listener;
pub use listener::*;

#[derive(Clone, Copy, PartialEq, Eq)]
struct RawKey {
    pub(crate) vk: VIRTUAL_KEY,
    pub(crate) make: u16, // Scan 1 Make code: https://learn.microsoft.com/en-us/windows/win32/inputdev/about-keyboard-input#scan-codes
    pub(crate) extended: bool,
}

impl Debug for RawKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawKey")
            .field("vk", &format_args!("{:#X}", self.vk.0))
            .field("make", &format_args!("{:#X}", self.make))
            .field("extended", &self.extended)
            .finish()
    }
}

impl TryFrom<RawKey> for Code {
    type Error = KeybindsError;

    fn try_from(value: RawKey) -> Result<Self, Self::Error> {
        use Code::*;
        use windows::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL;
        // mapping based on Standard "102" keyboard layout: https://w3c.github.io/uievents-code/#keyboard-102
        // and Scan 1 Make codes: https://learn.microsoft.com/en-us/windows/win32/inputdev/about-keyboard-input#scan-codes
        // as some applications/drivers might not pick up scan code only emits for special (well-known) keys,
        // we're emitting the corresponding virtual key for those instead since they are keyboard-layout agnostic
        Ok(match value.make {
            // Alphanumerical section
            // Row E
            0x29 => Backquote,
            0x02 => Digit1,
            0x03 => Digit2,
            0x04 => Digit3,
            0x05 => Digit4,
            0x06 => Digit5,
            0x07 => Digit6,
            0x08 => Digit7,
            0x09 => Digit8,
            0x0A => Digit9,
            0x0B => Digit0,
            0x0C => Minus,
            0x0D => Equal,
            0x0E => Backspace,
            // Row D
            0x0F => Tab,
            0x10 => {
                if value.extended {
                    MediaTrackPrevious
                } else {
                    KeyQ
                }
            }
            0x11 => KeyW,
            0x12 => KeyE,
            0x13 => KeyR,
            0x14 => KeyT,
            0x15 => KeyY,
            0x16 => KeyU,
            0x17 => KeyI,
            0x18 => KeyO,
            0x19 => {
                if value.extended {
                    MediaTrackNext
                } else {
                    KeyP
                }
            }
            0x1A => BracketLeft,
            0x1B => BracketRight,
            0x2B => Backslash,
            // Row C
            0x3A => CapsLock,
            0x1E => KeyA,
            0x1F => KeyS,
            0x20 => {
                if value.extended {
                    AudioVolumeMute
                } else {
                    KeyD
                }
            }
            0x21 => KeyF,
            0x22 => {
                if value.extended {
                    MediaPlayPause
                } else {
                    KeyG
                }
            }
            0x23 => KeyH,
            0x24 => {
                if value.extended {
                    MediaStop
                } else {
                    KeyJ
                }
            }
            0x25 => KeyK,
            0x26 => KeyL,
            0x27 => Semicolon,
            0x28 => Quote,
            0x1C => {
                if value.extended {
                    NumpadEnter
                } else {
                    Enter
                }
            }
            // Row B
            0x2A => {
                if value.extended && value.vk == VIRTUAL_KEY(0xFF) {
                    // "fake" extended Shift triggered at the beginning of a PrintScreen sequence
                    PrintScreen
                } else {
                    ShiftLeft
                }
            }
            0x56 => IntlBackslash,
            0x2C => KeyZ,
            0x2D => KeyX,
            0x2E => {
                if value.extended {
                    AudioVolumeDown
                } else {
                    KeyC
                }
            }
            0x2F => KeyV,
            0x30 => {
                if value.extended {
                    AudioVolumeUp
                } else {
                    KeyB
                }
            }
            0x31 => KeyN,
            0x32 => {
                if value.extended {
                    BrowserHome
                } else {
                    KeyM
                }
            }
            0x33 => Comma,
            0x34 => Period,
            0x35 => {
                if value.extended {
                    NumpadDivide
                } else {
                    Slash
                }
            }
            0x36 => ShiftRight,
            // Row A
            0x1D => {
                if value.extended {
                    ControlRight
                } else {
                    ControlLeft
                }
            }
            0x5B if value.extended => MetaLeft,
            0x38 => {
                if value.extended {
                    if value.vk == VK_CONTROL {
                        ControlRight
                    } else {
                        AltRight
                    }
                } else {
                    AltLeft
                }
            }
            0x39 => Space,
            0x5C if value.extended => MetaRight,
            0x5D => ContextMenu,

            // Arrow pad section
            // Control pad section
            // Numpad section
            // Row E
            0x45 => {
                if value.extended {
                    Pause
                } else {
                    NumLock
                }
            }
            0x37 => {
                if value.extended {
                    PrintScreen
                } else {
                    NumpadMultiply
                }
            }
            0x4A => NumpadSubtract,
            // Row D
            0x47 => {
                if value.extended {
                    Home
                } else {
                    Numpad7
                }
            }
            0x48 => {
                if value.extended {
                    ArrowUp
                } else {
                    Numpad8
                }
            }
            0x49 => {
                if value.extended {
                    PageUp
                } else {
                    Numpad9
                }
            }
            0x4E => NumpadAdd,
            // Row C
            0x4B => {
                if value.extended {
                    ArrowLeft
                } else {
                    Numpad4
                }
            }
            0x4C => Numpad5,
            0x4D => {
                if value.extended {
                    ArrowRight
                } else {
                    Numpad6
                }
            }
            // Row B
            0x4F => {
                if value.extended {
                    End
                } else {
                    Numpad1
                }
            }
            0x50 => {
                if value.extended {
                    ArrowDown
                } else {
                    Numpad2
                }
            }
            0x51 => {
                if value.extended {
                    PageDown
                } else {
                    Numpad3
                }
            }
            // Row A
            0x52 => {
                if value.extended {
                    Insert
                } else {
                    Numpad0
                }
            }
            0x53 => {
                if value.extended {
                    Delete
                } else {
                    NumpadDecimal
                }
            }

            // Function section
            // Row K
            0x01 => Escape,
            0x3B => F1,
            0x3C => F2,
            0x3D => F3,
            0x3E => F4,
            0x3F => F5,
            0x40 => F6,
            0x41 => F7,
            0x42 => F8,
            0x43 => F9,
            0x44 => F10,
            0x57 => F11,
            0x58 => F12,
            0x54 => PrintScreen,
            0x46 => ScrollLock,
            // Hidden
            0x64 => F13,
            0x65 => {
                if value.extended {
                    BrowserSearch
                } else {
                    F14
                }
            }
            0x66 => {
                if value.extended {
                    BrowserFavorites
                } else {
                    F15
                }
            }
            0x67 => {
                if value.extended {
                    BrowserRefresh
                } else {
                    F16
                }
            }
            0x68 => {
                if value.extended {
                    BrowserStop
                } else {
                    F17
                }
            }
            0x69 => {
                if value.extended {
                    BrowserForward
                } else {
                    F18
                }
            }
            0x6A => {
                if value.extended {
                    BrowserBack
                } else {
                    F19
                }
            }
            0x6B => F20,
            0x6C => {
                if value.extended {
                    LaunchMail
                } else {
                    F21
                }
            }
            0x6D => {
                if value.extended {
                    LaunchControlPanel
                } else {
                    F22
                }
            }
            0x6E => F23,
            0x76 => F24,

            // Media keys
            0x5E if value.extended => Power,
            0x5F if value.extended => Sleep,
            0x63 if value.extended => WakeUp,

            _ => return Err(KeybindsError::UnrecognizedCode(format!("{:?}", value))),
        })
    }
}

impl TryFrom<Code> for RawKey {
    type Error = KeybindsError;

    fn try_from(value: Code) -> Result<Self, Self::Error> {
        use Code::*;
        use windows::Win32::UI::Input::KeyboardAndMouse::*;
        let sc = |make: u16, extended: bool| RawKey {
            vk: VIRTUAL_KEY(0),
            make,
            extended,
        };
        let vk = |vk: VIRTUAL_KEY, make: u16, extended: bool| RawKey { vk, make, extended };
        // mapping based on Standard "102" keyboard layout: https://w3c.github.io/uievents-code/#keyboard-102
        // and Scan 1 Make codes: https://learn.microsoft.com/en-us/windows/win32/inputdev/about-keyboard-input#scan-codes
        Ok(match value {
            // Alphanumerical section
            // Row E
            Backquote => sc(0x29, false),
            Digit1 => sc(0x02, false),
            Digit2 => sc(0x03, false),
            Digit3 => sc(0x04, false),
            Digit4 => sc(0x05, false),
            Digit5 => sc(0x06, false),
            Digit6 => sc(0x07, false),
            Digit7 => sc(0x08, false),
            Digit8 => sc(0x09, false),
            Digit9 => sc(0x0A, false),
            Digit0 => sc(0x0B, false),
            Minus => sc(0x0C, false),
            Equal => sc(0x0D, false),
            Backspace => sc(0x0E, false),
            // Row D
            Tab => vk(VK_TAB, 0x0F, false),
            KeyQ => sc(0x10, false),
            KeyW => sc(0x11, false),
            KeyE => sc(0x12, false),
            KeyR => sc(0x13, false),
            KeyT => sc(0x14, false),
            KeyY => sc(0x15, false),
            KeyU => sc(0x16, false),
            KeyI => sc(0x17, false),
            KeyO => sc(0x18, false),
            KeyP => sc(0x19, false),
            BracketLeft => sc(0x1A, false),
            BracketRight => sc(0x1B, false),
            Backslash => sc(0x2B, false),
            // Row C
            CapsLock => vk(VK_CAPITAL, 0x3A, false),
            KeyA => sc(0x1E, false),
            KeyS => sc(0x1F, false),
            KeyD => sc(0x20, false),
            KeyF => sc(0x21, false),
            KeyG => sc(0x22, false),
            KeyH => sc(0x23, false),
            KeyJ => sc(0x24, false),
            KeyK => sc(0x25, false),
            KeyL => sc(0x26, false),
            Semicolon => sc(0x27, false),
            Quote => sc(0x28, false),
            Enter => sc(0x1C, false),
            NumpadEnter => sc(0x1C, true),
            // Row B
            NumpadMultiply => sc(0x37, false),
            PrintScreen => vk(VK_SNAPSHOT, 0x37, true),
            ShiftLeft => sc(0x2A, false),
            IntlBackslash => sc(0x56, false),
            KeyZ => sc(0x2C, false),
            KeyX => sc(0x2D, false),
            KeyC => sc(0x2E, false),
            KeyV => sc(0x2F, false),
            KeyB => sc(0x30, false),
            KeyN => sc(0x31, false),
            KeyM => sc(0x32, false),
            Comma => sc(0x33, false),
            Period => sc(0x34, false),
            Slash => sc(0x35, false),
            NumpadDivide => sc(0x35, true),
            ShiftRight => sc(0x36, false),
            // Row A
            ControlLeft => sc(0x1D, false),
            ControlRight => sc(0x1D, true),
            MetaLeft => sc(0x5B, true),
            AltLeft => sc(0x38, false),
            AltRight => sc(0x38, true),
            Space => sc(0x39, false),
            MetaRight => sc(0x5C, true),
            ContextMenu => sc(0x5D, true),

            // Arrow pad section
            // Row B
            ArrowUp => vk(VK_UP, 0x48, true),
            // Row A
            ArrowLeft => vk(VK_LEFT, 0x4B, true),
            ArrowDown => vk(VK_DOWN, 0x50, true),
            ArrowRight => vk(VK_RIGHT, 0x4D, true),

            // Control pad section
            // Numpad section
            // Row E
            NumLock => sc(0x45, false),
            Pause => vk(VK_PAUSE, 0x45, false),
            NumpadSubtract => sc(0x4A, false),
            // Row D
            Numpad7 => sc(0x47, false),
            Home => vk(VK_HOME, 0x47, true),
            Numpad8 => sc(0x48, false),
            Numpad9 => sc(0x49, false),
            PageUp => vk(VK_PRIOR, 0x49, true),
            NumpadAdd => sc(0x4E, false),
            // Row C
            Numpad4 => sc(0x4B, false),
            Numpad5 => sc(0x4C, false),
            Numpad6 => sc(0x4D, false),
            // Row B
            Numpad1 => sc(0x4F, false),
            End => vk(VK_END, 0x4F, true),
            Numpad2 => sc(0x50, false),
            Numpad3 => sc(0x51, false),
            PageDown => vk(VK_NEXT, 0x51, true),
            // Row A
            Numpad0 => sc(0x52, false),
            Insert => vk(VK_INSERT, 0x52, true),
            NumpadDecimal => sc(0x53, false),
            Delete => vk(VK_DELETE, 0x53, true),

            // Function section
            // Row K
            Escape => vk(VK_ESCAPE, 0x01, false),
            F1 => vk(VK_F1, 0x3B, false),
            F2 => vk(VK_F2, 0x3C, false),
            F3 => vk(VK_F3, 0x3D, false),
            F4 => vk(VK_F4, 0x3E, false),
            F5 => vk(VK_F5, 0x3F, false),
            F6 => vk(VK_F6, 0x40, false),
            F7 => vk(VK_F7, 0x41, false),
            F8 => vk(VK_F8, 0x42, false),
            F9 => vk(VK_F9, 0x43, false),
            F10 => vk(VK_F10, 0x44, false),
            F11 => vk(VK_F11, 0x57, false),
            F12 => vk(VK_F12, 0x58, false),
            ScrollLock => vk(VK_SCROLL, 0x46, false),
            // Hidden
            F13 => vk(VK_F13, 0x64, false),
            F14 => vk(VK_F14, 0x65, false),
            F15 => vk(VK_F15, 0x66, false),
            F16 => vk(VK_F16, 0x67, false),
            F17 => vk(VK_F17, 0x68, false),
            F18 => vk(VK_F18, 0x69, false),
            F19 => vk(VK_F19, 0x6A, false),
            F20 => vk(VK_F20, 0x6B, false),
            F21 => vk(VK_F21, 0x6C, false),
            F22 => vk(VK_F22, 0x6D, false),
            F23 => vk(VK_F23, 0x6E, false),
            F24 => vk(VK_F24, 0x76, false),

            // Media keys
            BrowserBack => vk(VK_BROWSER_BACK, 0x6A, true),
            BrowserFavorites => vk(VK_BROWSER_FAVORITES, 0x66, true),
            BrowserForward => vk(VK_BROWSER_FORWARD, 0x69, true),
            BrowserHome => vk(VK_BROWSER_HOME, 0x32, true),
            BrowserRefresh => vk(VK_BROWSER_REFRESH, 0x67, true),
            BrowserSearch => vk(VK_BROWSER_SEARCH, 0x65, true),
            BrowserStop => vk(VK_BROWSER_STOP, 0x68, true),
            LaunchControlPanel => vk(VK_LAUNCH_APP1, 0x6D, true),
            LaunchMail => vk(VK_LAUNCH_MAIL, 0x6C, true),
            MediaPlayPause => vk(VK_MEDIA_PLAY_PAUSE, 0x22, true),
            MediaStop => vk(VK_MEDIA_STOP, 0x24, true),
            MediaTrackNext => vk(VK_MEDIA_NEXT_TRACK, 0x19, true),
            MediaTrackPrevious => vk(VK_MEDIA_PREV_TRACK, 0x10, true),
            Power => sc(0x5E, true),
            Sleep => vk(VK_SLEEP, 0x5F, true),
            WakeUp => sc(0x63, true),
            AudioVolumeDown => vk(VK_VOLUME_DOWN, 0x2E, true),
            AudioVolumeMute => vk(VK_VOLUME_MUTE, 0x20, true),
            AudioVolumeUp => vk(VK_VOLUME_UP, 0x30, true),

            _ => return Err(KeybindsError::UnrecognizedCode(format!("{:?}", value))),
        })
    }
}
