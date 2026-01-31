pub mod manager;
pub mod session;

pub use manager::*;
pub use session::*;

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ClientManagerError {
    #[error("client with ID {0} already exists")]
    DuplicateClient(String),
    #[error("failed to send message: {0}")]
    MessageSendError(String),
}

pub type Result<T, E = ClientManagerError> = std::result::Result<T, E>;
