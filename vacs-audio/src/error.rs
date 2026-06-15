use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Audio device is not available, check if it is plugged in properly")]
    DeviceNotAvailable,
    #[error("Unsupported audio configuration, try a different audio device")]
    UnsupportedConfig,
    #[error("Audio device is busy or access was denied")]
    DeviceBusyOrDenied,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<cpal::Error> for AudioError {
    fn from(e: cpal::Error) -> Self {
        use cpal::ErrorKind::*;

        match e.kind() {
            DeviceNotAvailable | StreamInvalidated => AudioError::DeviceNotAvailable,
            UnsupportedConfig | InvalidInput => AudioError::UnsupportedConfig,
            DeviceBusy | PermissionDenied => AudioError::DeviceBusyOrDenied,
            _ => {
                tracing::warn!(err = ?e, "Received unmapped cpal error");
                AudioError::Other(e.into())
            }
        }
    }
}
