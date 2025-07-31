pub mod mock;
pub mod tokio;

use ::tokio::sync::mpsc;
use crate::error::SignalingError;
use async_trait::async_trait;
use tokio_tungstenite::tungstenite;
use vacs_protocol::ws::SignalingMessage;

#[async_trait]
pub trait SignalingSender: Send + Sync {
    async fn send(&mut self, msg: tungstenite::Message) -> Result<(), SignalingError>;
    async fn close(&mut self) -> Result<(), SignalingError>;
}

#[async_trait]
pub trait SignalingReceiver: Send + Sync {
    async fn recv(&mut self, send_tx: &mpsc::Sender<tungstenite::Message>) -> Result<SignalingMessage, SignalingError>;
}
