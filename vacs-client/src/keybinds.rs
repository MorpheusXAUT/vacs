use keyboard_types::{Code, KeyState};
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
