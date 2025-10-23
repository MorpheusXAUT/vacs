use crate::keybinds::KeybindsError;
use crate::keybinds::runtime::KeybindEmitter;
use crate::keybinds::runtime::windows::RawKey;
use keyboard_types::{Code, KeyState};
use std::fmt::{Debug, Formatter};
use windows::Win32::Foundation::GetLastError;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, SendInput,
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
        Self::send_code(code, state)
    }
}

impl WindowsKeybindEmitter {
    fn send_code(code: Code, state: KeyState) -> Result<(), KeybindsError> {
        let raw_key: RawKey = code.try_into()?;

        let mut flags = KEYBD_EVENT_FLAGS(0);
        if raw_key.vk.0 == 0 {
            flags |= KEYEVENTF_SCANCODE;
        }
        if raw_key.extended {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        if state.is_up() {
            flags |= KEYEVENTF_KEYUP;
        }

        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: raw_key.vk,
                    wScan: raw_key.make,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        log::trace!(
            "{code:?} -> {raw_key:?} -> {:?} {state:?}",
            InputDbg(&input)
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

#[repr(transparent)]
struct InputDbg<'a>(&'a INPUT);

impl<'a> Debug for InputDbg<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let input = self.0;
        match input.r#type {
            INPUT_KEYBOARD => {
                let ki = unsafe { input.Anonymous.ki };

                let mut flags = Vec::new();
                if ki.dwFlags.contains(KEYEVENTF_SCANCODE) {
                    flags.push("SCANCODE");
                }
                if ki.dwFlags.contains(KEYEVENTF_EXTENDEDKEY) {
                    flags.push("EXTENDEDKEY");
                }
                if ki.dwFlags.contains(KEYEVENTF_KEYUP) {
                    flags.push("KEYUP");
                }
                let flags = if flags.is_empty() {
                    format!("{:#X}", ki.dwFlags.0)
                } else {
                    format!("{:#X} [{}]", ki.dwFlags.0, flags.join("|"))
                };

                f.debug_struct("INPUT")
                    .field("type", &"INPUT_KEYBOARD")
                    .field("wVk", &format_args!("{:#X}", ki.wVk.0))
                    .field("wScan", &format_args!("{:#X}", ki.wScan))
                    .field("dwFlags", &flags)
                    .finish_non_exhaustive()
            }
            other => f
                .debug_struct("INPUT")
                .field("r#type", &other)
                .finish_non_exhaustive(),
        }
    }
}
