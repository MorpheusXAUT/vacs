use crate::keybinds::KeybindsError;
use crate::keybinds::runtime::KeybindEmitter;
use crate::keybinds::runtime::windows::RawKey;
use keyboard_types::{Code, KeyState};
use windows::Win32::Foundation::GetLastError;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    KEYEVENTF_SCANCODE, SendInput, VIRTUAL_KEY,
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
        log::trace!("Sending scan code {raw_key:?} ({code:?}) {state:?}");
        Self::send_scan_code(raw_key.make, raw_key.extended, state)
    }
}

impl WindowsKeybindEmitter {
    fn send_scan_code(
        scan_code: u16,
        extended: bool,
        key_state: KeyState,
    ) -> Result<(), KeybindsError> {
        let mut flags = KEYEVENTF_SCANCODE;
        if extended {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        if key_state.is_up() {
            flags |= KEYEVENTF_KEYUP;
        }

        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: scan_code,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let sent = unsafe { SendInput(&[input], size_of::<INPUT>() as i32) };
        if sent == 1 {
            Ok(())
        } else {
            Err(KeybindsError::Emitter(format!(
                "SendInput failed: {:?}",
                unsafe { GetLastError() }
            )))
        }
    }
}
