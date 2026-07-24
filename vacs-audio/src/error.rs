use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Audio device is not available, check if it is plugged in properly")]
    DeviceNotAvailable,
    #[error("Unsupported audio configuration, try a different audio device")]
    UnsupportedConfig,
    #[error("Audio device is busy or access was denied")]
    DeviceBusyOrDenied,
    #[error("Audio stream stopped because the audio device changed")]
    StreamInvalidated,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<cpal::Error> for AudioError {
    fn from(e: cpal::Error) -> Self {
        use cpal::ErrorKind::*;

        match e.kind() {
            DeviceNotAvailable => AudioError::DeviceNotAvailable,
            // Reported on default-following streams when the system default
            // device changes; the stream is dead and must be rebuilt.
            StreamInvalidated => AudioError::StreamInvalidated,
            UnsupportedConfig | InvalidInput => AudioError::UnsupportedConfig,
            DeviceBusy | PermissionDenied => AudioError::DeviceBusyOrDenied,
            // Transient dropped-samples events; the stream error callbacks
            // filter these out before they reach recovery logic. Handle explicitly
            // to avoid duplicate logging below.
            Xrun => AudioError::Other(e.into()),
            // Platform errors without a specific mapping (e.g. ALSA EIO from a
            // plugin during spin-up). Potentially transient: cpal workers keep
            // the stream running after reporting these.
            BackendError => AudioError::Other(e.into()),
            _ => {
                tracing::warn!(err = ?e, "Received unmapped cpal error");
                AudioError::Other(e.into())
            }
        }
    }
}
