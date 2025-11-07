use crate::keybinds::{KeyEvent, KeybindsError};
use keyboard_types::{Code, KeyState};
use objc2_core_graphics::{CGEvent, CGEventField, CGEventFlags, CGEventType};

mod emitter;
pub use emitter::*;

mod listener;
pub use listener::*;

struct KeyEventConverter {
    prev_flags: CGEventFlags,
}

impl KeyEventConverter {
    pub fn new() -> Self {
        Self {
            prev_flags: CGEventFlags::empty(),
        }
    }

    pub fn event_to_key_event(
        &mut self,
        event_type: CGEventType,
        event: &CGEvent,
    ) -> Result<KeyEvent, KeybindsError> {
        match event_type {
            CGEventType::KeyDown | CGEventType::KeyUp => {
                let state = if event_type == CGEventType::KeyDown {
                    KeyState::Down
                } else {
                    KeyState::Up
                };
                let keycode =
                    CGEvent::integer_value_field(Some(event), CGEventField::KeyboardEventKeycode);
                let code = cg_keycode_to_code(keycode)?;
                Ok(KeyEvent {
                    code,
                    label: code.to_string(),
                    state,
                })
            }
            CGEventType::FlagsChanged => {
                let keycode =
                    CGEvent::integer_value_field(Some(event), CGEventField::KeyboardEventKeycode);
                let current_flags = CGEvent::flags(Some(event));

                let (code, flag_bit) = match keycode {
                    0x38 => (Code::ShiftLeft, CGEventFlags::MaskShift),
                    0x3C => (Code::ShiftRight, CGEventFlags::MaskShift),
                    0x3B => (Code::ControlLeft, CGEventFlags::MaskControl),
                    0x3E => (Code::ControlRight, CGEventFlags::MaskControl),
                    0x3A => (Code::AltLeft, CGEventFlags::MaskAlternate),
                    0x3D => (Code::AltRight, CGEventFlags::MaskAlternate),
                    0x37 => (Code::MetaLeft, CGEventFlags::MaskCommand),
                    0x36 => (Code::MetaRight, CGEventFlags::MaskCommand),
                    0x39 => (Code::CapsLock, CGEventFlags::MaskAlphaShift),
                    _ => return Err(KeybindsError::Other("Unknown modifier keycode".to_string())),
                };

                let state = if current_flags.contains(flag_bit) {
                    KeyState::Down
                } else {
                    KeyState::Up
                };

                if state == KeyState::Down {
                    self.prev_flags.insert(flag_bit);
                } else {
                    self.prev_flags.remove(flag_bit);
                }

                Ok(KeyEvent {
                    code,
                    label: "".to_string(),
                    state,
                })
            }
            _ => Err(KeybindsError::Other("Unexpected event type".to_string())),
        }
    }
}

fn cg_keycode_to_code(keycode: i64) -> Result<Code, KeybindsError> {
    // https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_code_values#code_values_on_mac
    match keycode {
        0x00 => Ok(Code::KeyA),
        0x01 => Ok(Code::KeyS),
        0x02 => Ok(Code::KeyD),
        0x03 => Ok(Code::KeyF),
        0x04 => Ok(Code::KeyH),
        0x05 => Ok(Code::KeyG),
        0x06 => Ok(Code::KeyZ),
        0x07 => Ok(Code::KeyX),
        0x08 => Ok(Code::KeyC),
        0x09 => Ok(Code::KeyV),
        0x0A => Ok(Code::IntlBackslash),
        0x0B => Ok(Code::KeyB),
        0x0C => Ok(Code::KeyQ),
        0x0D => Ok(Code::KeyW),
        0x0E => Ok(Code::KeyE),
        0x0F => Ok(Code::KeyR),

        0x10 => Ok(Code::KeyY),
        0x11 => Ok(Code::KeyT),
        0x12 => Ok(Code::Digit1),
        0x13 => Ok(Code::Digit2),
        0x14 => Ok(Code::Digit3),
        0x15 => Ok(Code::Digit4),
        0x16 => Ok(Code::Digit6),
        0x17 => Ok(Code::Digit5),
        0x18 => Ok(Code::Equal),
        0x19 => Ok(Code::Digit9),
        0x1A => Ok(Code::Digit7),
        0x1B => Ok(Code::Minus),
        0x1C => Ok(Code::Digit8),
        0x1D => Ok(Code::Digit0),
        0x1E => Ok(Code::BracketRight),
        0x1F => Ok(Code::KeyO),

        0x20 => Ok(Code::KeyU),
        0x21 => Ok(Code::BracketLeft),
        0x22 => Ok(Code::KeyI),
        0x23 => Ok(Code::KeyP),
        0x24 => Ok(Code::Enter),
        0x25 => Ok(Code::KeyL),
        0x26 => Ok(Code::KeyJ),
        0x27 => Ok(Code::Quote),
        0x28 => Ok(Code::KeyK),
        0x29 => Ok(Code::Semicolon),
        0x2A => Ok(Code::Backslash),
        0x2B => Ok(Code::Comma),
        0x2C => Ok(Code::Slash),
        0x2D => Ok(Code::KeyN),
        0x2E => Ok(Code::KeyM),
        0x2F => Ok(Code::Period),

        0x30 => Ok(Code::Tab),
        0x31 => Ok(Code::Space),
        0x32 => Ok(Code::Backquote),
        0x33 => Ok(Code::Backspace),
        0x34 => Ok(Code::Enter),
        0x35 => Ok(Code::Escape),
        0x36 => Ok(Code::MetaRight),
        0x37 => Ok(Code::MetaLeft),
        0x38 => Ok(Code::ShiftLeft),
        0x39 => Ok(Code::CapsLock),
        0x3A => Ok(Code::AltLeft),
        0x3B => Ok(Code::ControlLeft),
        0x3C => Ok(Code::ShiftRight),
        0x3D => Ok(Code::AltRight),
        0x3E => Ok(Code::ControlRight),
        0x3F => Ok(Code::Fn),

        0x40 => Ok(Code::F17),
        0x41 => Ok(Code::NumpadDecimal),
        0x43 => Ok(Code::NumpadMultiply),
        0x45 => Ok(Code::NumpadAdd),
        0x47 => Ok(Code::NumLock),
        0x48 => Ok(Code::AudioVolumeUp),
        0x49 => Ok(Code::AudioVolumeDown),
        0x4A => Ok(Code::AudioVolumeMute),
        0x4B => Ok(Code::NumpadDivide),
        0x4C => Ok(Code::NumpadEnter),
        0x4E => Ok(Code::NumpadSubtract),
        0x4F => Ok(Code::F18),

        0x50 => Ok(Code::F19),
        0x51 => Ok(Code::NumpadEqual),
        0x52 => Ok(Code::Numpad0),
        0x53 => Ok(Code::Numpad1),
        0x54 => Ok(Code::Numpad2),
        0x55 => Ok(Code::Numpad3),
        0x56 => Ok(Code::Numpad4),
        0x57 => Ok(Code::Numpad5),
        0x58 => Ok(Code::Numpad6),
        0x59 => Ok(Code::Numpad7),
        0x5A => Ok(Code::F20),
        0x5B => Ok(Code::Numpad8),
        0x5C => Ok(Code::Numpad9),
        0x5D => Ok(Code::IntlYen),
        0x5E => Ok(Code::IntlRo),
        0x5F => Ok(Code::NumpadComma),

        0x60 => Ok(Code::F5),
        0x61 => Ok(Code::F6),
        0x62 => Ok(Code::F7),
        0x63 => Ok(Code::F3),
        0x64 => Ok(Code::F8),
        0x65 => Ok(Code::F9),
        0x66 => Ok(Code::Lang2),
        0x67 => Ok(Code::F11),
        0x68 => Ok(Code::Lang1),
        0x69 => Ok(Code::F13),
        0x6A => Ok(Code::F16),
        0x6B => Ok(Code::F14),
        0x6D => Ok(Code::F10),
        0x6E => Ok(Code::ContextMenu),
        0x6F => Ok(Code::F12),

        0x71 => Ok(Code::F15),
        0x72 => Ok(Code::Insert),
        0x73 => Ok(Code::Home),
        0x74 => Ok(Code::PageUp),
        0x75 => Ok(Code::Delete),
        0x76 => Ok(Code::F4),
        0x77 => Ok(Code::End),
        0x78 => Ok(Code::F2),
        0x79 => Ok(Code::PageDown),
        0x7A => Ok(Code::F1),
        0x7B => Ok(Code::ArrowLeft),
        0x7C => Ok(Code::ArrowRight),
        0x7D => Ok(Code::ArrowDown),
        0x7E => Ok(Code::ArrowUp),
        0x7F => Ok(Code::Power),

        _ => Err(KeybindsError::UnrecognizedCode(format!("{keycode}"))),
    }
}

fn code_to_cg_keycode(value: Code) -> Result<u16, KeybindsError> {
    Ok(match value {
        Code::KeyA => 0x00,
        Code::KeyS => 0x01,
        Code::KeyD => 0x02,
        Code::KeyF => 0x03,
        Code::KeyH => 0x04,
        Code::KeyG => 0x05,
        Code::KeyZ => 0x06,
        Code::KeyX => 0x07,
        Code::KeyC => 0x08,
        Code::KeyV => 0x09,
        Code::IntlBackslash => 0x0A,
        Code::KeyB => 0x0B,
        Code::KeyQ => 0x0C,
        Code::KeyW => 0x0D,
        Code::KeyE => 0x0E,
        Code::KeyR => 0x0F,

        Code::KeyY => 0x10,
        Code::KeyT => 0x11,
        Code::Digit1 => 0x12,
        Code::Digit2 => 0x13,
        Code::Digit3 => 0x14,
        Code::Digit4 => 0x15,
        Code::Digit6 => 0x16,
        Code::Digit5 => 0x17,
        Code::Equal => 0x18,
        Code::Digit9 => 0x19,
        Code::Digit7 => 0x1A,
        Code::Minus => 0x1B,
        Code::Digit8 => 0x1C,
        Code::Digit0 => 0x1D,
        Code::BracketRight => 0x1E,
        Code::KeyO => 0x1F,

        Code::KeyU => 0x20,
        Code::BracketLeft => 0x21,
        Code::KeyI => 0x22,
        Code::KeyP => 0x23,
        Code::Enter => 0x24,
        Code::KeyL => 0x25,
        Code::KeyJ => 0x26,
        Code::Quote => 0x27,
        Code::KeyK => 0x28,
        Code::Semicolon => 0x29,
        Code::Backslash => 0x2A,
        Code::Comma => 0x2B,
        Code::Slash => 0x2C,
        Code::KeyN => 0x2D,
        Code::KeyM => 0x2E,
        Code::Period => 0x2F,

        Code::Tab => 0x30,
        Code::Space => 0x31,
        Code::Backquote => 0x32,
        Code::Backspace => 0x33,
        Code::Escape => 0x35,
        Code::MetaRight => 0x36,
        Code::MetaLeft => 0x37,
        Code::ShiftLeft => 0x38,
        Code::CapsLock => 0x39,
        Code::AltLeft => 0x3A,
        Code::ControlLeft => 0x3B,
        Code::ShiftRight => 0x3C,
        Code::AltRight => 0x3D,
        Code::ControlRight => 0x3E,
        Code::Fn => 0x3F,

        Code::F17 => 0x40,
        Code::NumpadDecimal => 0x41,
        Code::NumpadMultiply => 0x43,
        Code::NumpadAdd => 0x45,
        Code::NumLock => 0x47,
        Code::AudioVolumeUp => 0x48,
        Code::AudioVolumeDown => 0x49,
        Code::AudioVolumeMute => 0x4A,
        Code::NumpadDivide => 0x4B,
        Code::NumpadEnter => 0x4C,
        Code::NumpadSubtract => 0x4E,
        Code::F18 => 0x4F,

        Code::F19 => 0x50,
        Code::NumpadEqual => 0x51,
        Code::Numpad0 => 0x52,
        Code::Numpad1 => 0x53,
        Code::Numpad2 => 0x54,
        Code::Numpad3 => 0x55,
        Code::Numpad4 => 0x56,
        Code::Numpad5 => 0x57,
        Code::Numpad6 => 0x58,
        Code::Numpad7 => 0x59,
        Code::F20 => 0x5A,
        Code::Numpad8 => 0x5B,
        Code::Numpad9 => 0x5C,
        Code::IntlYen => 0x5D,
        Code::IntlRo => 0x5E,
        Code::NumpadComma => 0x5F,

        Code::F5 => 0x60,
        Code::F6 => 0x61,
        Code::F7 => 0x62,
        Code::F3 => 0x63,
        Code::F8 => 0x64,
        Code::F9 => 0x65,
        Code::Lang2 => 0x66,
        Code::F11 => 0x67,
        Code::Lang1 => 0x68,
        Code::F13 => 0x69,
        Code::F16 => 0x6A,
        Code::F14 => 0x6B,
        Code::F10 => 0x6D,
        Code::ContextMenu => 0x6E,
        Code::F12 => 0x6F,

        Code::F15 => 0x71,
        Code::Insert => 0x72,
        Code::Home => 0x73,
        Code::PageUp => 0x74,
        Code::Delete => 0x75,
        Code::F4 => 0x76,
        Code::End => 0x77,
        Code::F2 => 0x78,
        Code::PageDown => 0x79,
        Code::F1 => 0x7A,
        Code::ArrowLeft => 0x7B,
        Code::ArrowRight => 0x7C,
        Code::ArrowDown => 0x7D,
        Code::ArrowUp => 0x7E,
        Code::Power => 0x7F,

        _ => return Err(KeybindsError::UnrecognizedCode(format!("{:?}", value))),
    })
}
