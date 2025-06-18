use crate::error::SignalingError;
use crate::transport::SignalingTransport;
use async_trait::async_trait;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite};
use vacs_protocol::SignalingMessage;

pub struct TokioTransport {
    websocket_tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::Message>,
    websocket_rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl TokioTransport {
    #[tracing::instrument(level = "info")]
    pub async fn new(url: &str) -> Result<Self, SignalingError> {
        tracing::info!("Connecting to signaling server");
        let (websocket_stream, response) =
            tokio_tungstenite::connect_async(url).await.map_err(|err| {
                tracing::error!(?err, "Failed to connect to signaling server");
                SignalingError::ConnectionError(err)
            })?;
        tracing::debug!(?response, "WebSocket handshake response");

        let (websocket_tx, websocket_rx) = websocket_stream.split();

        tracing::info!("Successfully established connection to signaling server");
        Ok(Self {
            websocket_tx,
            websocket_rx,
        })
    }
}

#[async_trait]
impl SignalingTransport for TokioTransport {
    #[tracing::instrument(level = "debug", skip(self, msg))]
    async fn send(&mut self, msg: SignalingMessage) -> Result<(), SignalingError> {
        let serialized = SignalingMessage::serialize(&msg).map_err(|err| {
            tracing::warn!(?err, "Failed to serialize message");
            SignalingError::SerializationError(err)
        })?;

        tracing::debug!("Sending message");
        self.websocket_tx
            .send(tungstenite::Message::from(serialized))
            .await
            .map_err(|err| {
                tracing::warn!(?err, "Failed to send message");
                SignalingError::Transport(anyhow::anyhow!(err))
            })?;

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn recv(&mut self) -> Result<SignalingMessage, SignalingError> {
        while let Some(msg) = self.websocket_rx.next().await {
            match msg {
                Ok(tungstenite::Message::Text(text)) => {
                    tracing::debug!("Received message");
                    return SignalingMessage::deserialize(&text).map_err(|err| {
                        tracing::warn!(?err, "Failed to deserialize message");
                        SignalingError::SerializationError(err)
                    });
                }
                Ok(tungstenite::Message::Close(reason)) => {
                    tracing::warn!(?reason, "Received Close WebSocket frame");
                    return Err(SignalingError::Disconnected);
                }
                Ok(other) => {
                    tracing::debug!(?other, "Skipping non-text WebSocket frame");
                }
                Err(err) => {
                    tracing::warn!(?err, "Failed to receive message");
                    return Err(SignalingError::Transport(anyhow::anyhow!(err)));
                }
            }
        }
        tracing::warn!("WebSocket stream closed");
        Err(SignalingError::Disconnected)
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn close(&mut self) -> Result<(), SignalingError> {
        self.websocket_tx.close().await.map_err(|err| {
            tracing::warn!(?err, "Failed to close WebSocket connection");
            SignalingError::Transport(anyhow::anyhow!(err))
        })
    }
}
