#[cfg(feature = "test-utils")]
pub mod mock;
pub mod tokio;

use crate::error::{SignalingError, SignalingRuntimeError};
use ::tokio::sync::mpsc;
use async_trait::async_trait;
use tokio_tungstenite::tungstenite;
use vacs_protocol::ws::server::ServerMessage;

#[async_trait]
pub trait SignalingTransport: Send + Sync + 'static {
    type Sender: SignalingSender;
    type Receiver: SignalingReceiver;

    async fn connect(&self) -> Result<(Self::Sender, Self::Receiver), SignalingError>;
}

#[async_trait]
pub trait SignalingSender: Send + Sync + 'static {
    async fn send(&mut self, msg: tungstenite::Message) -> Result<(), SignalingRuntimeError>;
    async fn close(&mut self) -> Result<(), SignalingRuntimeError>;
}

#[async_trait]
pub trait SignalingReceiver: Send + Sync + 'static {
    async fn recv(
        &mut self,
        send_tx: &mpsc::Sender<tungstenite::Message>,
    ) -> Result<ServerMessage, SignalingRuntimeError>;
}
