use crate::error::SignalingError;
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
                SignalingError::ConnectionError(err.into())
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
    async fn send(&mut self, msg: tungstenite::Message) -> Result<(), SignalingError> {
        if !matches!(msg, tungstenite::Message::Pong(_)) {
            tracing::trace!("Sending message to server");
        }
        self.websocket_tx.send(msg).await.map_err(|err| {
            tracing::warn!(?err, "Failed to send message");
            SignalingError::Transport(anyhow::anyhow!(err).into())
        })?;

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self), err)]
    async fn close(&mut self) -> Result<(), SignalingError> {
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
            SignalingError::Transport(anyhow::anyhow!(err).into())
        })
    }
}

#[async_trait]
impl SignalingReceiver for TokioReceiver {
    #[tracing::instrument(level = "debug", skip(self, send_tx), err)]
    async fn recv(
        &mut self,
        send_tx: &mpsc::Sender<tungstenite::Message>,
    ) -> Result<SignalingMessage, SignalingError> {
        while let Some(msg) = self.websocket_rx.next().await {
            match msg {
                Ok(tungstenite::Message::Text(text)) => {
                    tracing::debug!("Received message");
                    return SignalingMessage::deserialize(&text).map_err(|err| {
                        tracing::warn!(?err, "Failed to deserialize message");
                        SignalingError::SerializationError(err.into())
                    });
                }
                Ok(tungstenite::Message::Close(reason)) => {
                    tracing::warn!(?reason, "Received Close WebSocket frame");
                    return Err(SignalingError::Disconnected);
                }
                Ok(tungstenite::Message::Ping(data)) => {
                    if let Err(err) = send_tx.send(tungstenite::Message::Pong(data)).await {
                        tracing::warn!(?err, "Failed to send tokio Pong");
                        return Err(SignalingError::Disconnected);
                    }
                }
                Ok(other) => {
                    tracing::debug!(?other, "Skipping non-text WebSocket frame");
                }
                Err(err) => {
                    tracing::warn!(?err, "Failed to receive message");
                    return Err(SignalingError::Transport(anyhow::anyhow!(err).into()));
                }
            }
        }
        tracing::warn!("WebSocket stream closed");
        Err(SignalingError::Disconnected)
    }
}
