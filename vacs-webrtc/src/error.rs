use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebrtcError {
    #[error("Call Active")]
    CallActive,
    #[error("No call active")]
    NoCallActive,
    #[error(transparent)]
    Other(#[from] Box<anyhow::Error>),
}

impl From<anyhow::Error> for WebrtcError {
    fn from(err: anyhow::Error) -> Self {
        WebrtcError::Other(Box::new(err))
    }
}
