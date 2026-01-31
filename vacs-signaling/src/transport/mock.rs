use crate::error::{SignalingError, SignalingRuntimeError, TransportFailureReason};
use crate::transport::{SignalingReceiver, SignalingSender, SignalingTransport};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::tungstenite;
use tokio_util::sync::CancellationToken;
use vacs_protocol::ws::server::ServerMessage;

pub struct MockTransport {
    pub outgoing_rx: broadcast::Receiver<tungstenite::Message>,
    pub outgoing_tx: broadcast::Sender<tungstenite::Message>,
    pub incoming_tx: broadcast::Sender<tungstenite::Message>,
    pub incoming_rx: broadcast::Receiver<tungstenite::Message>,
    pub ready: Arc<tokio::sync::Notify>,
    pub disconnect_token: CancellationToken,
}

impl Default for MockTransport {
    fn default() -> Self {
        let (outgoing_tx, outgoing_rx) = broadcast::channel(32);
        let (incoming_tx, incoming_rx) = broadcast::channel(32);
        Self {
            outgoing_tx,
            outgoing_rx,
            incoming_tx,
            incoming_rx,
            ready: Arc::new(tokio::sync::Notify::new()),
            disconnect_token: CancellationToken::new(),
        }
    }
}

impl MockTransport {
    pub fn disconnect_token(&self) -> CancellationToken {
        self.disconnect_token.clone()
    }
}

#[async_trait]
impl SignalingTransport for MockTransport {
    type Sender = MockSender;
    type Receiver = MockReceiver;

    async fn connect(&self) -> Result<(Self::Sender, Self::Receiver), SignalingError> {
        let sender = MockSender {
            tx: Some(self.outgoing_tx.clone()),
            disconnect_token: self.disconnect_token.child_token(),
        };
        let receiver = MockReceiver {
            rx: self.incoming_tx.subscribe(),
            disconnect_token: self.disconnect_token.child_token(),
        };

        self.ready.notify_one();

        Ok((sender, receiver))
    }
}

pub struct MockSender {
    tx: Option<broadcast::Sender<tungstenite::Message>>,
    disconnect_token: CancellationToken,
}

pub struct MockReceiver {
    rx: broadcast::Receiver<tungstenite::Message>,
    disconnect_token: CancellationToken,
}

#[async_trait]
impl SignalingSender for MockSender {
    #[tracing::instrument(level = "debug", skip(self))]
    async fn send(&mut self, msg: tungstenite::Message) -> Result<(), SignalingRuntimeError> {
        tracing::debug!("Sending SignalingMessage");
        if self.disconnect_token.is_cancelled() {
            return Err(SignalingRuntimeError::Transport(
                TransportFailureReason::Send("Sender closed".to_string()),
            ));
        }

        if let Some(ref tx) = self.tx {
            tx.send(msg).map_err(|err| {
                tracing::warn!(?err, "Failed to send SignalingMessage");
                SignalingRuntimeError::Transport(TransportFailureReason::Send(err.to_string()))
            })?;
            Ok(())
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
    ) -> Result<ServerMessage, SignalingRuntimeError> {
        loop {
            tokio::select! {
                biased;
                _ = self.disconnect_token.cancelled() => {
                    tracing::warn!("Channel closed");
                    return Err(SignalingRuntimeError::Disconnected(None));
                }
                msg = self.rx.recv() => {
                    tracing::debug!(?msg, "Received tungstenite::Message");
                    match msg {
                        Ok(tungstenite::Message::Text(text)) => {
                            tracing::debug!("Received message");
                            return ServerMessage::deserialize(&text).map_err(|err| {
                                tracing::warn!(?err, "Failed to deserialize message");
                                SignalingRuntimeError::SerializationError(err.to_string())
                            });
                        }
                        Ok(tungstenite::Message::Close(reason)) => {
                            tracing::warn!(?reason, "Received Close WebSocket frame");
                            return Err(SignalingRuntimeError::Disconnected(None));
                        }
                        Ok(tungstenite::Message::Ping(data)) => {
                            if let Err(err) = send_tx.send(tungstenite::Message::Pong(data)).await {
                                tracing::warn!(?err, "Failed to send mock Pong");
                                return Err(SignalingRuntimeError::Disconnected(None));
                            }
                        }
                        Ok(other) => {
                            tracing::debug!(?other, "Skipping non-text WebSocket frame");
                        }
                        Err(_) => {
                            tracing::warn!("Channel closed");
                            return Err(SignalingRuntimeError::Disconnected(None));
                        }
                    }
                }
            }
        }
    }
}
