//! X11 keybind listener implementation.
//!
//! # Keycode Mapping
//!
//! On every evdev-based X server (Xorg and Xwayland alike), X keycodes are Linux
//! evdev keycodes shifted by a fixed offset of 8. The table below therefore maps
//! evdev keycodes to their W3C [`Code`] equivalents and is shared by the listener
//! (X keycode -> `Code`) and, in reverse, by the emitter (`Code` -> X keycode).
//!
//! Exotic keys missing from the table are logged and ignored by the listener and
//! rejected by the emitter.

mod listener;

pub use listener::*;

use crate::keybinds::KeybindsError;
use keyboard_types::Code;

/// Offset between X11 keycodes and Linux evdev keycodes on evdev-based X servers.
const EVDEV_KEYCODE_OFFSET: u32 = 8;

/// Linux evdev keycode (`KEY_*` from `input-event-codes.h`) to W3C code mapping.
const EVDEV_CODE_MAP: &[(u32, Code)] = &[
    (1, Code::Escape),
    (2, Code::Digit1),
    (3, Code::Digit2),
    (4, Code::Digit3),
    (5, Code::Digit4),
    (6, Code::Digit5),
    (7, Code::Digit6),
    (8, Code::Digit7),
    (9, Code::Digit8),
    (10, Code::Digit9),
    (11, Code::Digit0),
    (12, Code::Minus),
    (13, Code::Equal),
    (14, Code::Backspace),
    (15, Code::Tab),
    (16, Code::KeyQ),
    (17, Code::KeyW),
    (18, Code::KeyE),
    (19, Code::KeyR),
    (20, Code::KeyT),
    (21, Code::KeyY),
    (22, Code::KeyU),
    (23, Code::KeyI),
    (24, Code::KeyO),
    (25, Code::KeyP),
    (26, Code::BracketLeft),
    (27, Code::BracketRight),
    (28, Code::Enter),
    (29, Code::ControlLeft),
    (30, Code::KeyA),
    (31, Code::KeyS),
    (32, Code::KeyD),
    (33, Code::KeyF),
    (34, Code::KeyG),
    (35, Code::KeyH),
    (36, Code::KeyJ),
    (37, Code::KeyK),
    (38, Code::KeyL),
    (39, Code::Semicolon),
    (40, Code::Quote),
    (41, Code::Backquote),
    (42, Code::ShiftLeft),
    (43, Code::Backslash),
    (44, Code::KeyZ),
    (45, Code::KeyX),
    (46, Code::KeyC),
    (47, Code::KeyV),
    (48, Code::KeyB),
    (49, Code::KeyN),
    (50, Code::KeyM),
    (51, Code::Comma),
    (52, Code::Period),
    (53, Code::Slash),
    (54, Code::ShiftRight),
    (55, Code::NumpadMultiply),
    (56, Code::AltLeft),
    (57, Code::Space),
    (58, Code::CapsLock),
    (59, Code::F1),
    (60, Code::F2),
    (61, Code::F3),
    (62, Code::F4),
    (63, Code::F5),
    (64, Code::F6),
    (65, Code::F7),
    (66, Code::F8),
    (67, Code::F9),
    (68, Code::F10),
    (69, Code::NumLock),
    (70, Code::ScrollLock),
    (71, Code::Numpad7),
    (72, Code::Numpad8),
    (73, Code::Numpad9),
    (74, Code::NumpadSubtract),
    (75, Code::Numpad4),
    (76, Code::Numpad5),
    (77, Code::Numpad6),
    (78, Code::NumpadAdd),
    (79, Code::Numpad1),
    (80, Code::Numpad2),
    (81, Code::Numpad3),
    (82, Code::Numpad0),
    (83, Code::NumpadDecimal),
    (86, Code::IntlBackslash),
    (87, Code::F11),
    (88, Code::F12),
    (89, Code::IntlRo),
    (96, Code::NumpadEnter),
    (97, Code::ControlRight),
    (98, Code::NumpadDivide),
    (99, Code::PrintScreen),
    (100, Code::AltRight),
    (102, Code::Home),
    (103, Code::ArrowUp),
    (104, Code::PageUp),
    (105, Code::ArrowLeft),
    (106, Code::ArrowRight),
    (107, Code::End),
    (108, Code::ArrowDown),
    (109, Code::PageDown),
    (110, Code::Insert),
    (111, Code::Delete),
    (113, Code::AudioVolumeMute),
    (114, Code::AudioVolumeDown),
    (115, Code::AudioVolumeUp),
    (117, Code::NumpadEqual),
    (119, Code::Pause),
    (121, Code::NumpadComma),
    (124, Code::IntlYen),
    (125, Code::MetaLeft),
    (126, Code::MetaRight),
    (127, Code::ContextMenu),
    (163, Code::MediaTrackNext),
    (164, Code::MediaPlayPause),
    (165, Code::MediaTrackPrevious),
    (166, Code::MediaStop),
    (183, Code::F13),
    (184, Code::F14),
    (185, Code::F15),
    (186, Code::F16),
    (187, Code::F17),
    (188, Code::F18),
    (189, Code::F19),
    (190, Code::F20),
    (191, Code::F21),
    (192, Code::F22),
    (193, Code::F23),
    (194, Code::F24),
];

/// Translate an X11 keycode into a W3C [`Code`].
pub(super) fn x_keycode_to_code(keycode: u32) -> Result<Code, KeybindsError> {
    keycode
        .checked_sub(EVDEV_KEYCODE_OFFSET)
        .and_then(|evdev| {
            EVDEV_CODE_MAP
                .iter()
                .find(|(key, _)| *key == evdev)
                .map(|(_, code)| *code)
        })
        .ok_or_else(|| KeybindsError::UnrecognizedCode(format!("X11 keycode {keycode}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evdev_map_has_no_duplicates() {
        for (i, (key, code)) in EVDEV_CODE_MAP.iter().enumerate() {
            for (other_key, other_code) in &EVDEV_CODE_MAP[i + 1..] {
                assert_ne!(key, other_key, "duplicate evdev keycode {key}");
                assert_ne!(code, other_code, "duplicate code {code}");
            }
        }
    }

    #[test]
    fn x_keycode_translation() {
        assert_eq!(x_keycode_to_code(38).unwrap(), Code::KeyA);
        assert_eq!(x_keycode_to_code(65).unwrap(), Code::Space);
        assert!(x_keycode_to_code(5).is_err()); // below the evdev offset
        assert!(x_keycode_to_code(9999).is_err());
    }
}
