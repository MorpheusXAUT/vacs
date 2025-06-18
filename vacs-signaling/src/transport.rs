pub mod mock;
pub mod tokio;

use crate::error::SignalingError;
use async_trait::async_trait;
use vacs_protocol::SignalingMessage;

#[async_trait]
pub trait SignalingTransport: Send + Sync {
    async fn send(&mut self, msg: SignalingMessage) -> Result<(), SignalingError>;
    async fn recv(&mut self) -> Result<SignalingMessage, SignalingError>;
    async fn close(&mut self) -> Result<(), SignalingError>;
}
