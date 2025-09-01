use serde::Serialize;
use serde_json::Value;
use std::fmt::Debug;
use tauri::{AppHandle, Emitter};
use thiserror::Error;
use vacs_protocol::ws::CallErrorReason;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unauthorized")]
    Unauthorized,
    #[error(transparent)]
    AudioDevice(#[from] Box<vacs_audio::error::AudioError>),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Signaling error: {0}")]
    Signaling(#[from] Box<vacs_signaling::error::SignalingError>),
    #[error("HTTP error: {0}")]
    Reqwest(#[from] Box<reqwest::Error>),
    #[error("WebRTC error: {0}")]
    Webrtc(#[from] Box<vacs_webrtc::error::WebrtcError>),
    #[error(transparent)]
    Other(#[from] Box<anyhow::Error>),
}

impl From<vacs_audio::error::AudioError> for Error {
    fn from(err: vacs_audio::error::AudioError) -> Self {
        Error::AudioDevice(Box::new(err))
    }
}

impl From<vacs_signaling::error::SignalingError> for Error {
    fn from(err: vacs_signaling::error::SignalingError) -> Self {
        Error::Signaling(Box::new(err))
    }
}

impl From<vacs_webrtc::error::WebrtcError> for Error {
    fn from(err: vacs_webrtc::error::WebrtcError) -> Self {
        Error::Webrtc(Box::new(err))
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Reqwest(Box::new(err))
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Other(Box::new(err))
    }
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        FrontendError::from(self).serialize(serializer)
    }
}

pub trait HandleUnauthorizedExt<R> {
    fn handle_unauthorized(self, app: &AppHandle) -> Result<R, Error>;
}

impl<R> HandleUnauthorizedExt<R> for Result<R, Error> {
    fn handle_unauthorized(self, app: &AppHandle) -> Result<R, Error> {
        match self {
            Ok(val) => Ok(val),
            Err(Error::Unauthorized) => {
                log::info!("Not authenticated");
                app.emit("auth:unauthenticated", Value::Null).ok();
                Err(Error::Unauthorized)
            }
            Err(err) => Err(err),
        }
    }
}

pub trait LogErrExt<R> {
    #[allow(dead_code)]
    fn log_err(self) -> Result<R, Error>;
}

impl<R> LogErrExt<R> for Result<R, Error> {
    fn log_err(self) -> Result<R, Error> {
        match self {
            Ok(val) => Ok(val),
            Err(err) => {
                log::error!("{err:?}");
                Err(err)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FrontendError {
    title: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout_ms: Option<u16>,
}

impl FrontendError {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            timeout_ms: None,
        }
    }

    pub fn new_with_timeout(
        title: impl Into<String>,
        message: impl Into<String>,
        timeout_ms: u16,
    ) -> Self {
        Self::new(title, message).with_timeout(timeout_ms)
    }

    pub fn with_timeout(mut self, timeout_ms: u16) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

impl From<Error> for FrontendError {
    fn from(err: Error) -> Self {
        FrontendError::from(&err)
    }
}

impl From<&Error> for FrontendError {
    fn from(err: &Error) -> Self {
        match err {
            Error::Unauthorized => FrontendError::new_with_timeout(
                "Unauthorized",
                "Your authentication expired. Please log in again.",
                5000,
            ),
            Error::AudioDevice(err) => FrontendError::new("Audio device error", err.to_string()),
            Error::Reqwest(err) => FrontendError::new("HTTP error", err.to_string()),
            Error::Network(err) => FrontendError::new("Network error", err),
            Error::Signaling(err) => FrontendError::new("Signaling error", err.to_string()),
            Error::Webrtc(err) => FrontendError::new("WebRTC error", err.to_string()),
            Error::Other(err) => FrontendError::new("Error", err.to_string()),
        }
    }
}

impl From<Error> for CallErrorReason {
    fn from(err: Error) -> Self {
        match err {
            Error::AudioDevice(_) => CallErrorReason::AudioFailure,
            Error::Webrtc(err) => match err.as_ref() {
                vacs_webrtc::error::WebrtcError::CallActive => CallErrorReason::CallFailure,
                vacs_webrtc::error::WebrtcError::NoCallActive => CallErrorReason::CallFailure,
                _ => CallErrorReason::WebrtcFailure,
            },
            _ => CallErrorReason::Other,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallError {
    peer_id: String,
    reason: String,
}

impl CallError {
    pub fn new(peer_id: String, is_local: bool, reason: CallErrorReason) -> Self {
        Self {
            peer_id,
            reason: format!(
                "{} {}",
                if is_local { "Local" } else { "Remote" },
                match reason {
                    CallErrorReason::WebrtcFailure => "Connection failure",
                    CallErrorReason::AudioFailure => "Audio failure",
                    CallErrorReason::CallFailure => "Call failure",
                    CallErrorReason::SignalingFailure => "Target not reachable",
                    CallErrorReason::Other => "Unknown failure",
                }
            ),
        }
    }
}
