use crate::keybinds::runtime::{KeybindEmitter, KeybindListener};
use crate::keybinds::{KeyEvent, KeybindsError};
use keyboard_types::{Code, KeyState};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
#[allow(dead_code)]
pub struct NoopKeybindListener;

impl KeybindListener for NoopKeybindListener {
    async fn start(_key_event_tx: UnboundedSender<KeyEvent>) -> Result<Self, KeybindsError>
    where
        Self: Sized,
    {
        log::warn!(
            "No global keyboard listener available on this platform, using stub noop implementation. Keyboard keybinds will not work; joystick bindings are unaffected."
        );
        Ok(Self)
    }
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct NoopKeybindEmitter;

impl KeybindEmitter for NoopKeybindEmitter {
    fn start() -> Result<Self, KeybindsError>
    where
        Self: Sized,
    {
        log::warn!(
            "No keybind emitter available on this platform, using stub noop implementation. Emitting keys to external applications (AudioForVatsim radio integration) will not work."
        );
        Ok(Self)
    }

    fn emit(&self, _code: Code, _state: KeyState) -> Result<(), KeybindsError> {
        Ok(())
    }
}
