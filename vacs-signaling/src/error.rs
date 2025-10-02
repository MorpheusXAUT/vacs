use thiserror::Error;
use tokio_tungstenite::tungstenite;
use vacs_protocol::ws::{DisconnectReason, ErrorReason, LoginFailureReason};

#[derive(Debug, Error)]
pub enum SignalingError {
    #[error("login failed: {0:?}")]
    LoginError(LoginFailureReason),
    #[error("transport error: {0}")]
    Transport(#[from] Box<tungstenite::error::Error>),
    #[error("signaling protocol error: {0}")]
    ProtocolError(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("runtime error: {0:?}")]
    Runtime(SignalingRuntimeError),
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone, Error)]
pub enum SignalingRuntimeError {
    #[error("disconnected: {0:?}")]
    Disconnected(Option<DisconnectReason>),
    #[error("reconnect failed: {0:?}")]
    ReconnectFailed(ReconnectFailureReason),
    #[error("server error: {0:?}")]
    ServerError(ErrorReason),
    #[error("transport error: {0:?}")]
    Transport(TransportFailureReason),
    #[error("serialization error: {0}")]
    SerializationError(String),
}

impl SignalingRuntimeError {
    pub fn can_reconnect(&self) -> bool {
        matches!(self, SignalingRuntimeError::Disconnected(reason) if reason.is_none())
            || matches!(
                self,
                SignalingRuntimeError::ServerError(_)
                    | SignalingRuntimeError::Transport(_)
                    | SignalingRuntimeError::SerializationError(_)
            )
    }

    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            SignalingRuntimeError::Disconnected(_)
                | SignalingRuntimeError::ReconnectFailed(_)
                | SignalingRuntimeError::ServerError(_)
                | SignalingRuntimeError::Transport(_)
        )
    }
}

impl From<SignalingRuntimeError> for SignalingError {
    fn from(value: SignalingRuntimeError) -> Self {
        SignalingError::Runtime(value)
    }
}

#[derive(Debug, Clone)]
pub enum ReconnectFailureReason {
    Connection,
    Login(LoginFailureReason),
    Other(String),
}

#[derive(Debug, Clone)]
pub enum TransportFailureReason {
    Send(String),
    Receive(String),
    Close(String),
}

impl From<SignalingError> for ReconnectFailureReason {
    fn from(value: SignalingError) -> ReconnectFailureReason {
        match value {
            SignalingError::LoginError(reason) => ReconnectFailureReason::Login(reason),
            SignalingError::Transport(_) => ReconnectFailureReason::Connection,
            SignalingError::ProtocolError(reason) => ReconnectFailureReason::Other(reason),
            SignalingError::Timeout(reason) => ReconnectFailureReason::Other(reason),
            SignalingError::Runtime(error) => match error {
                SignalingRuntimeError::Disconnected(_)
                | SignalingRuntimeError::ServerError(_)
                | SignalingRuntimeError::SerializationError(_) => {
                    ReconnectFailureReason::Connection
                }
                _ => {
                    unreachable!("SignalingRuntimeError is not valid as ReconnectFailureReason");
                }
            },
            SignalingError::Other(reason) => ReconnectFailureReason::Other(reason),
        }
    }
}
