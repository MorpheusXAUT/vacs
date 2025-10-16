use crate::config::TransmitConfig;
use crate::error::Error;
use crate::keybinds::engine::KeybindEngineHandle;
use keyboard_types::{Code, KeyState};
use tauri::{AppHandle, Manager};
use thiserror::Error;

pub mod commands;
pub mod engine;
pub mod runtime;

#[derive(Debug, Clone, Error)]
pub enum KeybindsError {
    #[error("Keybinds runtime error: {0}")]
    Runtime(String),
    #[error("Unrecognized keybinds code: {0}")]
    UnrecognizedCode(String),
    #[error("Missing keybind for selected transmit mode")]
    MissingKeybind,
    #[error("Other keybinds error: {0}")]
    Other(String),
}

pub type KeyEvent = (Code, KeyState);

pub trait KeybindsTrait {
    fn register_keybinds(&self, app: AppHandle) -> Result<(), Error>;
    fn unregister_keybinds(&self, app: AppHandle);
}

impl KeybindsTrait for TransmitConfig {
    fn register_keybinds(&self, app: AppHandle) -> Result<(), Error> {
        app.state::<KeybindEngineHandle>().lock().set_config(self)
    }

    fn unregister_keybinds(&self, app: AppHandle) {
        app.state::<KeybindEngineHandle>().lock().stop();
    }
}
