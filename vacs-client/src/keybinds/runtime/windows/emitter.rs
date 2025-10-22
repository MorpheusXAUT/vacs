use crate::keybinds::KeybindsError;
use crate::keybinds::runtime::KeybindEmitter;
use crate::keybinds::runtime::windows::RawKey;
use keyboard_types::{Code, KeyState};
use std::fmt::Debug;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, SendInput, VIRTUAL_KEY,
};

#[derive(Debug)]
pub struct WindowsKeybindEmitter;

impl KeybindEmitter for WindowsKeybindEmitter {
    fn start() -> Result<Self, KeybindsError>
    where
        Self: Sized,
    {
        Ok(Self)
    }

    fn emit(&self, code: Code, state: KeyState) -> Result<(), KeybindsError> {
        let raw_key: RawKey = code.try_into()?;
        log::trace!("Sending raw key {raw_key:?} ({code:?}) {state:?}");
        Self::send_raw_key(raw_key, state)
    }
}

impl WindowsKeybindEmitter {
    fn send_raw_key(raw_key: RawKey, state: KeyState) -> Result<(), KeybindsError> {
        let (vk, scan_code, flags) = if raw_key.vk.0 == 0 {
            // no VIRTUAL_KEY override defined, emit as scan code
            let mut f = KEYEVENTF_SCANCODE;
            if raw_key.extended {
                f |= KEYEVENTF_EXTENDEDKEY;
            }
            if state.is_up() {
                f |= KEYEVENTF_KEYUP;
            }
            (VIRTUAL_KEY(0), raw_key.make, f)
        } else {
            // VIRTUAL_KEY override defined, emit as virtual key
            (
                raw_key.vk,
                0u16,
                if state.is_up() {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
            )
        };

        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: scan_code,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        log::trace!(
            "Sending input wVk {vk:?}, wScan {scan_code}, dwFlags {flags:?} ({raw_key:?}) {state:?}"
        );
        if unsafe { SendInput(&[input], size_of::<INPUT>() as i32) } != 1 {
            return Err(KeybindsError::Emitter(format!(
                "SendInput failed: {:?}",
                unsafe { GetLastError() }
            )));
        }

        Ok(())
    }
}
