pub mod mock;
pub mod tokio;

use crate::error::SignalingError;
use ::tokio::sync::mpsc;
use async_trait::async_trait;
use tokio_tungstenite::tungstenite;
use vacs_protocol::ws::SignalingMessage;

#[async_trait]
pub trait SignalingTransport: Send + Sync {
    type Sender: SignalingSender;
    type Receiver: SignalingReceiver;

    async fn connect(&self) -> Result<(Self::Sender, Self::Receiver), SignalingError>;
}

#[async_trait]
pub trait SignalingSender: Send + Sync {
    async fn send(&mut self, msg: tungstenite::Message) -> Result<(), SignalingError>;
    async fn close(&mut self) -> Result<(), SignalingError>;
}

#[async_trait]
pub trait SignalingReceiver: Send + Sync {
    async fn recv(
        &mut self,
        send_tx: &mpsc::Sender<tungstenite::Message>,
    ) -> Result<SignalingMessage, SignalingError>;
}
