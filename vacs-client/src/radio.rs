pub mod push_to_talk;

use keyboard_types::KeyState;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum RadioError {
    #[error("Radio integration error: {0}")]
    Integration(String),
    #[error("Radio transmit error: {0}")]
    Transmit(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub enum RadioIntegration {
    #[default]
    AudioForVatsim,
    TrackAudio,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TransmissionState {
    Active,
    Inactive,
}

impl From<TransmissionState> for KeyState {
    fn from(value: TransmissionState) -> Self {
        match value {
            TransmissionState::Active => KeyState::Down,
            TransmissionState::Inactive => KeyState::Up,
        }
    }
}

impl From<KeyState> for TransmissionState {
    fn from(value: KeyState) -> Self {
        match value {
            KeyState::Down => TransmissionState::Active,
            KeyState::Up => TransmissionState::Inactive,
        }
    }
}

pub trait Radio: Send + Sync + Debug + 'static {
    fn transmit(&self, state: TransmissionState) -> Result<(), RadioError>;
}

pub type DynRadio = Arc<dyn Radio>;
