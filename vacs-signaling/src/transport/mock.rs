use crate::error::{SignalingRuntimeError, TransportFailureReason};
use crate::transport::{SignalingReceiver, SignalingSender};
use async_trait::async_trait;
use tokio::sync::mpsc;
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

    (
        (
            MockSender {
                tx: Some(outgoing_tx),
            },
            MockReceiver { rx: incoming_rx },
        ),
        handle,
    )
}

#[async_trait]
impl SignalingSender for MockSender {
    #[tracing::instrument(level = "debug", skip(self))]
    async fn send(&mut self, msg: tungstenite::Message) -> Result<(), SignalingRuntimeError> {
        tracing::debug!("Sending SignalingMessage");
        if let Some(ref tx) = self.tx {
            tx.send(msg).await.map_err(|err| {
                tracing::warn!(?err, "Failed to send SignalingMessage");
                SignalingRuntimeError::Transport(TransportFailureReason::Send(err.to_string()))
            })
        } else {
            Err(SignalingRuntimeError::Transport(
                TransportFailureReason::Send("Sender closed".to_string()),
            ))
        }
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn close(&mut self) -> Result<(), SignalingRuntimeError> {
        tracing::debug!("Closing MockSender");
        self.tx = None;
        Ok(())
    }
}

#[async_trait]
impl SignalingReceiver for MockReceiver {
    #[tracing::instrument(level = "debug", skip_all, send_tx)]
    async fn recv(
        &mut self,
        send_tx: &mpsc::Sender<tungstenite::Message>,
    ) -> Result<SignalingMessage, SignalingRuntimeError> {
        while let Some(msg) = self.rx.recv().await {
            tracing::debug!(?msg, "Received tungstenite::Message");
            match msg {
                tungstenite::Message::Text(text) => {
                    tracing::debug!("Received message");
                    return SignalingMessage::deserialize(&text).map_err(|err| {
                        tracing::warn!(?err, "Failed to deserialize message");
                        SignalingRuntimeError::SerializationError(err.to_string())
                    });
                }
                tungstenite::Message::Close(reason) => {
                    tracing::warn!(?reason, "Received Close WebSocket frame");
                    return Err(SignalingRuntimeError::Disconnected);
                }
                tungstenite::Message::Ping(data) => {
                    if let Err(err) = send_tx.send(tungstenite::Message::Pong(data)).await {
                        tracing::warn!(?err, "Failed to send mock Pong");
                        return Err(SignalingRuntimeError::Disconnected);
                    }
                }
                other => {
                    tracing::debug!(?other, "Skipping non-text WebSocket frame");
                }
            }
        }
        tracing::warn!("Channel closed");
        Err(SignalingRuntimeError::Disconnected)
    }
}
