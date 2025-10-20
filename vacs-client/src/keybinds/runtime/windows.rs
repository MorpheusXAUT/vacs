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
            .field("make", &format_args!("{:#06X}", self.make))
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
        match value.make {
            // Alphanumerical section
            // Row E
            0x0029 => Ok(Backquote),
            0x0002 => Ok(Digit1),
            0x0003 => Ok(Digit2),
            0x0004 => Ok(Digit3),
            0x0005 => Ok(Digit4),
            0x0006 => Ok(Digit5),
            0x0007 => Ok(Digit6),
            0x0008 => Ok(Digit7),
            0x0009 => Ok(Digit8),
            0x000A => Ok(Digit9),
            0x000B => Ok(Digit0),
            0x000C => Ok(Minus),
            0x000D => Ok(Equal),
            0x000E => Ok(Backspace),
            // Row D
            0x000F => Ok(Tab),
            0x0010 => Ok(KeyQ),
            0x0011 => Ok(KeyW),
            0x0012 => Ok(KeyE),
            0x0013 => Ok(KeyR),
            0x0014 => Ok(KeyT),
            0x0015 => Ok(KeyY),
            0x0016 => Ok(KeyU),
            0x0017 => Ok(KeyI),
            0x0018 => Ok(KeyO),
            0x0019 => Ok(KeyP),
            0x001A => Ok(BracketLeft),
            0x001B => Ok(BracketRight),
            0x002B => Ok(Backslash),
            // Row C
            0x003A => Ok(CapsLock),
            0x001E => Ok(KeyA),
            0x001F => Ok(KeyS),
            0x0020 => Ok(KeyD),
            0x0021 => Ok(KeyF),
            0x0022 => Ok(KeyG),
            0x0023 => Ok(KeyH),
            0x0024 => Ok(KeyJ),
            0x0025 => Ok(KeyK),
            0x0026 => Ok(KeyL),
            0x0027 => Ok(Semicolon),
            0x0028 => Ok(Quote),
            0x001C => Ok(if value.extended { NumpadEnter } else { Enter }),
            // Row B
            0x002A => Ok(if value.extended && value.vk == VIRTUAL_KEY(0xFF) {
                // "fake" extended Shift triggered at the beginning of a PrintScreen sequence
                PrintScreen
            } else {
                ShiftLeft
            }),
            0x0056 => Ok(IntlBackslash),
            0x002C => Ok(KeyZ),
            0x002D => Ok(KeyX),
            0x002E => Ok(KeyC),
            0x002F => Ok(KeyV),
            0x0030 => Ok(KeyB),
            0x0031 => Ok(KeyN),
            0x0032 => Ok(KeyM),
            0x0033 => Ok(Comma),
            0x0034 => Ok(Period),
            0x0035 => Ok(if value.extended { NumpadDivide } else { Slash }),
            0x0036 => Ok(ShiftRight),
            // Row A
            0x001D => Ok(if value.extended {
                ControlRight
            } else {
                ControlLeft
            }),
            0x005B => Ok(MetaLeft),
            0x0038 => Ok(if value.extended {
                if value.vk == VK_CONTROL {
                    ControlRight
                } else {
                    AltRight
                }
            } else {
                AltLeft
            }),
            0x0039 => Ok(Space),
            0xE038 => Ok(AltRight),
            0x005C => Ok(MetaRight),
            0x005D => Ok(ContextMenu),
            0xE01D => Ok(ControlRight),

            // Arrow pad section
            // Row B
            0xE048 => Ok(ArrowUp),
            // Row A
            0xE04B => Ok(ArrowLeft),
            0xE050 => Ok(ArrowDown),
            0xE04D => Ok(ArrowRight),

            // Control pad section
            // Numpad section
            // Row E
            0x0045 | 0xE045 => Ok(NumLock),
            0x0037 => Ok(if value.extended {
                PrintScreen
            } else {
                NumpadMultiply
            }),
            0x004A => Ok(NumpadSubtract),
            // Row D
            0x0047 => Ok(if value.extended { Home } else { Numpad7 }),
            0x0048 => Ok(Numpad8),
            0x0049 => Ok(if value.extended { PageUp } else { Numpad9 }),
            0x004E => Ok(NumpadAdd),
            // Row C
            0x004B => Ok(Numpad4),
            0x004C => Ok(Numpad5),
            0x004D => Ok(Numpad6),
            // Row B
            0x004F => Ok(if value.extended { End } else { Numpad1 }),
            0x0050 => Ok(Numpad2),
            0x0051 => Ok(if value.extended { PageDown } else { Numpad3 }),
            // Row A
            0x0052 => Ok(if value.extended { Insert } else { Numpad0 }),
            0x0053 => Ok(if value.extended {
                Delete
            } else {
                NumpadDecimal
            }),

            // Function section
            // Row K
            0x0001 => Ok(Escape),
            0x003B => Ok(F1),
            0x003C => Ok(F2),
            0x003D => Ok(F3),
            0x003E => Ok(F4),
            0x003F => Ok(F5),
            0x0040 => Ok(F6),
            0x0041 => Ok(F7),
            0x0042 => Ok(F8),
            0x0043 => Ok(F9),
            0x0044 => Ok(F10),
            0x0057 => Ok(F11),
            0x0058 => Ok(F12),
            0xE037 | 0x0054 => Ok(PrintScreen),
            0x0046 => Ok(ScrollLock),
            0xE046 => Ok(Pause),
            // Hidden
            0x0064 => Ok(F13),
            0x0065 => Ok(F14),
            0x0066 => Ok(F15),
            0x0067 => Ok(F16),
            0x0068 => Ok(F17),
            0x0069 => Ok(F18),
            0x006A => Ok(F19),
            0x006B => Ok(F20),
            0x006C => Ok(F21),
            0x006D => Ok(F22),
            0x006E => Ok(F23),
            0x0076 => Ok(F24),

            // Media keys
            0xE06A => Ok(BrowserBack),
            0xE066 => Ok(BrowserFavorites),
            0xE069 => Ok(BrowserForward),
            0xE032 => Ok(BrowserHome),
            0xE067 => Ok(BrowserRefresh),
            0xE065 => Ok(BrowserSearch),
            0xE068 => Ok(BrowserStop),
            0xE06D => Ok(LaunchControlPanel),
            0xE06C => Ok(LaunchMail),
            0xE022 => Ok(MediaPlayPause),
            0xE024 => Ok(MediaStop),
            0xE019 => Ok(MediaTrackNext),
            0xE010 => Ok(MediaTrackPrevious),
            0xE05E => Ok(Power),
            0xE05F => Ok(Sleep),
            0xE063 => Ok(WakeUp),
            0xE02E => Ok(AudioVolumeDown),
            0xE020 => Ok(AudioVolumeMute),
            0xE030 => Ok(AudioVolumeUp),

            _ => Err(KeybindsError::UnrecognizedCode(format!("{:?}", value))),
        }
    }
}

impl TryFrom<Code> for RawKey {
    type Error = KeybindsError;

    fn try_from(value: Code) -> Result<Self, Self::Error> {
        use Code::*;
        fn rk(make: u16, extended: bool) -> Result<RawKey, KeybindsError> {
            Ok(RawKey {
                vk: VIRTUAL_KEY(0),
                make,
                extended,
            })
        }
        // mapping based on Standard "102" keyboard layout: https://w3c.github.io/uievents-code/#keyboard-102
        // and Scan 1 Make codes: https://learn.microsoft.com/en-us/windows/win32/inputdev/about-keyboard-input#scan-codes
        match value {
            // Alphanumerical section
            // Row E
            Backquote => rk(0x0029, false),
            Digit1 => rk(0x0002, false),
            Digit2 => rk(0x0003, false),
            Digit3 => rk(0x0004, false),
            Digit4 => rk(0x0005, false),
            Digit5 => rk(0x0006, false),
            Digit6 => rk(0x0007, false),
            Digit7 => rk(0x0008, false),
            Digit8 => rk(0x0009, false),
            Digit9 => rk(0x000A, false),
            Digit0 => rk(0x000B, false),
            Minus => rk(0x000C, false),
            Equal => rk(0x000D, false),
            Backspace => rk(0x000E, false),
            // Row D
            Tab => rk(0x000F, false),
            KeyQ => rk(0x0010, false),
            KeyW => rk(0x0011, false),
            KeyE => rk(0x0012, false),
            KeyR => rk(0x0013, false),
            KeyT => rk(0x0014, false),
            KeyY => rk(0x0015, false),
            KeyU => rk(0x0016, false),
            KeyI => rk(0x0017, false),
            KeyO => rk(0x0018, false),
            KeyP => rk(0x0019, false),
            BracketLeft => rk(0x001A, false),
            BracketRight => rk(0x001B, false),
            Backslash => rk(0x002B, false),
            // Row C
            CapsLock => rk(0x003A, false),
            KeyA => rk(0x001E, false),
            KeyS => rk(0x001F, false),
            KeyD => rk(0x0020, false),
            KeyF => rk(0x0021, false),
            KeyG => rk(0x0022, false),
            KeyH => rk(0x0023, false),
            KeyJ => rk(0x0024, false),
            KeyK => rk(0x0025, false),
            KeyL => rk(0x0026, false),
            Semicolon => rk(0x0027, false),
            Quote => rk(0x0028, false),
            Enter => rk(0x001C, false),
            NumpadEnter => rk(0x001C, true),
            // Row B
            NumpadMultiply => rk(0x0037, false),
            PrintScreen => rk(0x0037, true),
            ShiftLeft => rk(0x002A, false),
            IntlBackslash => rk(0x0056, false),
            KeyZ => rk(0x002C, false),
            KeyX => rk(0x002D, false),
            KeyC => rk(0x002E, false),
            KeyV => rk(0x002F, false),
            KeyB => rk(0x0030, false),
            KeyN => rk(0x0031, false),
            KeyM => rk(0x0032, false),
            Comma => rk(0x0033, false),
            Period => rk(0x0034, false),
            Slash => rk(0x0035, false),
            NumpadDivide => rk(0x0035, true),
            ShiftRight => rk(0x0036, false),
            // Row A
            ControlLeft => rk(0x001D, false),
            ControlRight => rk(0x001D, true),
            MetaLeft => rk(0x005B, false),
            AltLeft => rk(0x0038, false),
            AltRight => rk(0x0038, true),
            Space => rk(0x0039, false),
            MetaRight => rk(0x005C, false),
            ContextMenu => rk(0x005D, false),

            // Arrow pad section
            // Row B
            ArrowUp => rk(0xE048, false),
            // Row A
            ArrowLeft => rk(0xE04B, false),
            ArrowDown => rk(0xE050, false),
            ArrowRight => rk(0xE04D, false),

            // Control pad section
            // Numpad section
            // Row E
            NumLock => rk(0x0045, false),
            Pause => rk(0x0045, true),
            NumpadSubtract => rk(0x004A, false),
            // Row D
            Numpad7 => rk(0x0047, false),
            Home => rk(0x0047, true),
            Numpad8 => rk(0x0048, false),
            Numpad9 => rk(0x0049, false),
            PageUp => rk(0x0049, true),
            NumpadAdd => rk(0x004E, false),
            // Row C
            Numpad4 => rk(0x004B, false),
            Numpad5 => rk(0x004C, false),
            Numpad6 => rk(0x004D, false),
            // Row B
            Numpad1 => rk(0x004F, false),
            End => rk(0x004F, true),
            Numpad2 => rk(0x0050, false),
            Numpad3 => rk(0x0051, false),
            PageDown => rk(0x0051, true),
            // Row A
            Numpad0 => rk(0x0052, false),
            Insert => rk(0x0052, true),
            NumpadDecimal => rk(0x0053, false),
            Delete => rk(0x0053, true),

            // Function section
            // Row K
            Escape => rk(0x0001, false),
            F1 => rk(0x003B, false),
            F2 => rk(0x003C, false),
            F3 => rk(0x003D, false),
            F4 => rk(0x003E, false),
            F5 => rk(0x003F, false),
            F6 => rk(0x0040, false),
            F7 => rk(0x0041, false),
            F8 => rk(0x0042, false),
            F9 => rk(0x0043, false),
            F10 => rk(0x0044, false),
            F11 => rk(0x0057, false),
            F12 => rk(0x0058, false),
            ScrollLock => rk(0x0046, false),
            // Hidden
            F13 => rk(0x0064, false),
            F14 => rk(0x0065, false),
            F15 => rk(0x0066, false),
            F16 => rk(0x0067, false),
            F17 => rk(0x0068, false),
            F18 => rk(0x0069, false),
            F19 => rk(0x006A, false),
            F20 => rk(0x006B, false),
            F21 => rk(0x006C, false),
            F22 => rk(0x006D, false),
            F23 => rk(0x006E, false),
            F24 => rk(0x0076, false),

            // Media keys
            BrowserBack => rk(0xE06A, false),
            BrowserFavorites => rk(0xE066, false),
            BrowserForward => rk(0xE069, false),
            BrowserHome => rk(0xE032, false),
            BrowserRefresh => rk(0xE067, false),
            BrowserSearch => rk(0xE065, false),
            BrowserStop => rk(0xE068, false),
            LaunchControlPanel => rk(0xE06D, false),
            LaunchMail => rk(0xE06C, false),
            MediaPlayPause => rk(0xE022, false),
            MediaStop => rk(0xE024, false),
            MediaTrackNext => rk(0xE019, false),
            MediaTrackPrevious => rk(0xE010, false),
            Power => rk(0xE05E, false),
            Sleep => rk(0xE05F, false),
            WakeUp => rk(0xE063, false),
            AudioVolumeDown => rk(0xE02E, false),
            AudioVolumeMute => rk(0xE020, false),
            AudioVolumeUp => rk(0xE030, false),

            _ => Err(KeybindsError::UnrecognizedCode(format!("{:?}", value))),
        }
    }
}
