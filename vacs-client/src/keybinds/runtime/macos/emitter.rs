use crate::keybinds::KeybindsError;
use crate::keybinds::runtime::KeybindEmitter;
use crate::keybinds::runtime::macos::code_to_cg_keycode;
use keyboard_types::{Code, KeyState};
use objc2_core_graphics::{CGEvent, CGEventTapLocation};

#[derive(Debug)]
pub struct MacOsKeybindEmitter;

impl KeybindEmitter for MacOsKeybindEmitter {
    fn start() -> Result<Self, KeybindsError>
    where
        Self: Sized,
    {
        Ok(Self)
    }

    fn emit(&self, code: Code, state: KeyState) -> Result<(), KeybindsError> {
        let virtual_key = code_to_cg_keycode(code)?;

        let event = CGEvent::new_keyboard_event(None, virtual_key, state == KeyState::Down)
            .ok_or_else(|| KeybindsError::Emitter("Failed to create keyboard event".to_string()))?;

        log::trace!("{code:?} -> {virtual_key:#04X} {state:?}",);

        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(event.as_ref()));

        Ok(())
    }
}
