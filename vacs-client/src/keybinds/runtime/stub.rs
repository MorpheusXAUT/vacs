use crate::keybinds::runtime::{KeybindEmitter, KeybindListener};
use crate::keybinds::{KeyEvent, KeybindsError};
use keyboard_types::{Code, KeyState};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

#[derive(Debug)]
pub struct NoopKeybindListener {
    _tx: UnboundedSender<KeyEvent>,
}

impl KeybindListener for NoopKeybindListener {
    fn start() -> Result<(Self, UnboundedReceiver<KeyEvent>), KeybindsError>
    where
        Self: Sized,
    {
        log::warn!(
            "No keybind listener available, using stub noop implementation. Your selected keybinds will not work!"
        );
        let (tx, rx) = unbounded_channel();
        Ok((Self { _tx: tx }, rx))
    }

    fn stop(mut self) {}
}

#[derive(Debug, Default)]
pub struct NoopKeybindEmitter;

impl KeybindEmitter for NoopKeybindEmitter {
    fn start() -> Result<Self, KeybindsError>
    where
        Self: Sized,
    {
        log::warn!(
            "No keybind emitter available, using stub noop implementation. Your selected keybinds will not work!"
        );
        Ok(Self)
    }

    fn stop(mut self) {}

    fn emit(&self, _code: Code, _state: KeyState) -> Result<(), KeybindsError> {
        Ok(())
    }
}
