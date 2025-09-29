use crate::error::{SignalingError, SignalingRuntimeError, TransportFailureReason};
use crate::transport::{SignalingReceiver, SignalingSender, SignalingTransport};
use async_trait::async_trait;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite};
use vacs_protocol::ws::SignalingMessage;

#[derive(Debug, Clone)]
pub struct TokioTransport {
    url: String,
}

impl TokioTransport {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
        }
    }
}

#[async_trait]
impl SignalingTransport for TokioTransport {
    type Sender = TokioSender;
    type Receiver = TokioReceiver;

    #[tracing::instrument(level = "info", err)]
    async fn connect(&self) -> Result<(Self::Sender, Self::Receiver), SignalingError> {
        tracing::info!("Connecting to signaling server");
        let (websocket_stream, response) = tokio_tungstenite::connect_async(&self.url)
            .await
            .map_err(|err| {
                tracing::error!(?err, "Failed to connect to signaling server");
                SignalingError::Transport(err.into())
            })?;
        tracing::debug!(?response, "WebSocket handshake response");

        let (websocket_tx, websocket_rx) = websocket_stream.split();

        tracing::info!("Successfully established connection to signaling server");
        Ok((TokioSender { websocket_tx }, TokioReceiver { websocket_rx }))
    }
}

pub struct TokioSender {
    websocket_tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::Message>,
}

pub struct TokioReceiver {
    websocket_rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

#[async_trait]
impl SignalingSender for TokioSender {
    #[tracing::instrument(level = "debug", skip(self, msg), err)]
    async fn send(&mut self, msg: tungstenite::Message) -> Result<(), SignalingRuntimeError> {
        if !matches!(msg, tungstenite::Message::Pong(_)) {
            tracing::trace!("Sending message to server");
        }
        self.websocket_tx.send(msg).await.map_err(|err| {
            tracing::warn!(?err, "Failed to send message");
            SignalingRuntimeError::Transport(TransportFailureReason::Send(err.to_string()))
        })?;

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self), err)]
    async fn close(&mut self) -> Result<(), SignalingRuntimeError> {
        let _ = self
            .websocket_tx
            .send(tungstenite::Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "".into(),
            })))
            .await
            .inspect_err(|err| {
                tracing::warn!(?err, "Failed to send Close frame");
            });

        self.websocket_tx.close().await.map_err(|err| {
            tracing::warn!(?err, "Failed to close WebSocket connection");
            SignalingRuntimeError::Transport(TransportFailureReason::Close(err.to_string()))
        })
    }
}

#[async_trait]
impl SignalingReceiver for TokioReceiver {
    #[tracing::instrument(level = "debug", skip(self, send_tx), err)]
    async fn recv(
        &mut self,
        send_tx: &mpsc::Sender<tungstenite::Message>,
    ) -> Result<SignalingMessage, SignalingRuntimeError> {
        while let Some(msg) = self.websocket_rx.next().await {
            match msg {
                Ok(tungstenite::Message::Text(text)) => {
                    tracing::debug!("Received message");
                    return match SignalingMessage::deserialize(&text) {
                        Ok(SignalingMessage::Disconnected { reason }) => {
                            tracing::debug!(
                                ?reason,
                                "Received Disconnected message, returning disconnected error"
                            );
                            Err(SignalingRuntimeError::Disconnected(Some(reason)))
                        }
                        Ok(msg) => Ok(msg),
                        Err(err) => {
                            tracing::warn!(?err, "Failed to deserialize message");
                            Err(SignalingRuntimeError::SerializationError(err.to_string()))
                        }
                    };
                }
                Ok(tungstenite::Message::Close(reason)) => {
                    tracing::warn!(?reason, "Received Close WebSocket frame");
                    return Err(SignalingRuntimeError::Disconnected(None));
                }
                Ok(tungstenite::Message::Ping(data)) => {
                    if let Err(err) = send_tx.send(tungstenite::Message::Pong(data)).await {
                        tracing::warn!(?err, "Failed to send tokio Pong");
                        return Err(SignalingRuntimeError::Disconnected(None));
                    }
                }
                Ok(other) => {
                    tracing::debug!(?other, "Skipping non-text WebSocket frame");
                }
                Err(err) => {
                    tracing::warn!(?err, "Failed to receive message");
                    return Err(SignalingRuntimeError::Transport(
                        TransportFailureReason::Receive(err.to_string()),
                    ));
                }
            }
        }
        tracing::warn!("WebSocket stream closed");
        Err(SignalingRuntimeError::Disconnected(None))
    }
}
