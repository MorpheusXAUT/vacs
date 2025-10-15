use crate::keybinds::runtime::KeybindRuntime;
use crate::keybinds::{KeyEvent, KeybindsError};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

#[derive(Debug)]
pub struct NoopKeybindRuntime;

impl KeybindRuntime for NoopKeybindRuntime {
    fn start() -> Result<(Self, UnboundedReceiver<KeyEvent>), KeybindsError>
    where
        Self: Sized,
    {
        log::warn!(
            "No keybind runtime available, using stub noop implementation. Your selected keybinds will not work!"
        );
        let (_, rx) = unbounded_channel();
        Ok((Self, rx))
    }

    fn stop(&mut self) {}
}
