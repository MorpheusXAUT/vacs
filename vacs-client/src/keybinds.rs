use keyboard_types::{Code, KeyState};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod commands;
pub mod engine;
pub mod runtime;

#[derive(Debug, Clone, Error)]
pub enum KeybindsError {
    #[error("Keybinds listener error: {0}")]
    Listener(String),
    #[error("Keybinds emitter error: {0}")]
    Emitter(String),
    #[error("Unrecognized keybinds code: {0}")]
    UnrecognizedCode(String),
    #[error("Fake marker")]
    FakeMarker,
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone)]
pub struct KeyEvent {
    code: Code,
    #[allow(dead_code)]
    label: String,
    state: KeyState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Keybind {
    PushToTalk,
    PushToMute,
    RadioIntegration,
    AcceptCall,
    EndCall,
}
