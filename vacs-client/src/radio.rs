pub mod push_to_talk;
pub mod track_audio;

use keyboard_types::KeyState;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;
use tauri::Emitter;
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

/// Radio state representing the current operational status of the chosen radio integration.
#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq, Hash)]
pub enum RadioState {
    #[default]
    /// No radio integration configured.
    NotConfigured,

    /// Radio configured but not connected to backend.
    /// This includes initial connection attempts, reconnection attempts, and disconnected states.
    Disconnected,

    /// Connected to a radio backend, but the backend itself is not connected to VATSIM voice server.
    Connected,

    /// Connected to a radio backend, which is connected to the VATSIM voice server.
    VoiceConnected,

    /// Connected to a radio backend and monitoring at least one frequency (RX ready).
    RxIdle,

    /// Connected and receiving transmission from others.
    RxActive,

    /// Connected and actively transmitting.
    /// May or may not be receiving simultaneously (TX takes priority).
    TxActive,

    /// Fatal connection error or client error event.
    Error,
}

impl RadioState {
    pub fn emit(&self, app: &tauri::AppHandle) {
        log::trace!("Emitting radio state: {self:?}");
        app.emit("radio:state", self).ok();
    }
}

#[async_trait::async_trait]
pub trait Radio: Send + Sync + Debug + 'static {
    async fn transmit(&self, state: TransmissionState) -> Result<(), RadioError>;
    async fn reconnect(&self) -> Result<(), RadioError> {
        Ok(())
    }

    fn state(&self) -> RadioState;
}

pub type DynRadio = Arc<dyn Radio>;
