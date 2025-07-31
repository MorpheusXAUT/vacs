use crate::error::SignalingError;
use crate::transport::{SignalingSender, SignalingReceiver};
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;
use tokio_tungstenite::tungstenite;
use vacs_protocol::ws::SignalingMessage;

pub struct MockHandle {
    pub outgoing_rx: mpsc::Receiver<tungstenite::Message>,
    pub incoming_tx: mpsc::Sender<tungstenite::Message>,
}

pub struct MockSender {
    tx: Option<mpsc::Sender<tungstenite::Message>>,
}

pub struct MockReceiver {
    rx: mpsc::Receiver<tungstenite::Message>,
}

#[tracing::instrument(level = "info")]
pub fn create() -> ((MockSender, MockReceiver), MockHandle) {
    let (outgoing_tx, outgoing_rx) = mpsc::channel(32);
    let (incoming_tx, incoming_rx) = mpsc::channel(32);

    let handle = MockHandle {
        outgoing_rx,
        incoming_tx,
    };

    ((MockSender {tx: Some(outgoing_tx)}, MockReceiver {rx: incoming_rx}), handle)
}

#[async_trait]
impl SignalingSender for MockSender {
    #[tracing::instrument(level = "debug", skip(self))]
    async fn send(&mut self, msg: tungstenite::Message) -> Result<(), SignalingError> {
        tracing::debug!("Sending SignalingMessage");
        if let Some(ref tx) = self.tx {
            tx.send(msg).await.map_err(|err| {
                tracing::warn!(?err, "Failed to send SignalingMessage");
                SignalingError::Transport(anyhow::anyhow!(err))
            })
        } else {
            Err(SignalingError::Transport(anyhow::anyhow!("Sender closed")))
        }
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn close(&mut self) -> Result<(), SignalingError> {
        tracing::debug!("Closing MockSender");
        self.tx = None;
        Ok(())
    }
}

#[async_trait]
impl SignalingReceiver for MockReceiver {
    #[tracing::instrument(level = "debug", skip_all)]
    async fn recv(&mut self, send_tx: &mpsc::Sender<tungstenite::Message>) -> Result<SignalingMessage, SignalingError> {
        while let Some(msg) = self.rx.recv().await {
            tracing::debug!(?msg, "Received tungstenite::Message");
            match msg {
                tungstenite::Message::Text(text) => {
                    tracing::debug!("Received message");
                    return SignalingMessage::deserialize(&text).map_err(|err| {
                        tracing::warn!(?err, "Failed to deserialize message");
                        SignalingError::SerializationError(err)
                    });
                }
                tungstenite::Message::Close(reason) => {
                    tracing::warn!(?reason, "Received Close WebSocket frame");
                    return Err(SignalingError::Disconnected);
                }
                tungstenite::Message::Ping(data) => {
                    if let Err(err) = send_tx
                        .send(tungstenite::Message::Pong(data))
                        .await
                    {
                        tracing::warn!(?err, "Failed to send mock Pong");
                        return Err(SignalingError::Disconnected);
                    }
                }
                other => {
                    tracing::debug!(?other, "Skipping non-text WebSocket frame");
                }
            }
        }
        tracing::warn!("Channel closed");
        Err(SignalingError::Disconnected)
    }
}
