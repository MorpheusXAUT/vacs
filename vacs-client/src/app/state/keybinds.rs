use crate::app::state::{AppStateInner, sealed};
use crate::keybinds::engine::KeybindEngineHandle;

pub trait AppStateKeybindsExt: sealed::Sealed {
    fn keybind_engine_handle(&self) -> KeybindEngineHandle;
}

impl AppStateKeybindsExt for AppStateInner {
    fn keybind_engine_handle(&self) -> KeybindEngineHandle {
        self.keybind_engine.clone()
    }
}
