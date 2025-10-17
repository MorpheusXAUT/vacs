use crate::keybinds::runtime::KeybindRuntime;
use crate::keybinds::{KeyEvent, KeybindsError};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

#[derive(Debug)]
pub struct NoopKeybindRuntime {
    _tx: UnboundedSender<KeyEvent>,
}

impl KeybindRuntime for NoopKeybindRuntime {
    fn start() -> Result<(Self, UnboundedReceiver<KeyEvent>), KeybindsError>
    where
        Self: Sized,
    {
        log::warn!(
            "No keybind runtime available, using stub noop implementation. Your selected keybinds will not work!"
        );
        let (tx, rx) = unbounded_channel();
        Ok((Self { _tx: tx }, rx))
    }

    fn stop(&mut self) {}
}
