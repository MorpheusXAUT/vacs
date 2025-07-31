use crate::error::SignalingError;
use crate::transport::{SignalingReceiver, SignalingSender};
use async_trait::async_trait;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite};
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use vacs_protocol::ws::SignalingMessage;

pub struct TokioSender {
    websocket_tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::Message>,
}

pub struct TokioReceiver {
    websocket_rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

#[tracing::instrument(level = "info")]
pub async fn create(url: &str) -> Result<(TokioSender, TokioReceiver), SignalingError> {
    tracing::info!("Connecting to signaling server");
    let (websocket_stream, response) =
        tokio_tungstenite::connect_async(url).await.map_err(|err| {
            tracing::error!(?err, "Failed to connect to signaling server");
            SignalingError::ConnectionError(err)
        })?;
    tracing::debug!(?response, "WebSocket handshake response");

    let (websocket_tx, websocket_rx) = websocket_stream.split();

    tracing::info!("Successfully established connection to signaling server");
    Ok((TokioSender { websocket_tx }, TokioReceiver { websocket_rx }))
}

#[async_trait]
impl SignalingSender for TokioSender {
    #[tracing::instrument(level = "debug", skip(self, msg))]
    async fn send(&mut self, msg: tungstenite::Message) -> Result<(), SignalingError> {
        tracing::debug!("Sending message to server");
        self.websocket_tx
            .send(msg)
            .await
            .map_err(|err| {
                tracing::warn!(?err, "Failed to send message");
                SignalingError::Transport(anyhow::anyhow!(err))
            })?;

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn close(&mut self) -> Result<(), SignalingError> {
        let _ = self.websocket_tx.send(tungstenite::Message::Close(Some(CloseFrame{code: CloseCode::Normal, reason: "".into()}))).await.inspect_err(|err| {
            tracing::warn!(?err, "Failed to send Close frame");
        });

        self.websocket_tx.close().await.map_err(|err| {
            tracing::warn!(?err, "Failed to close WebSocket connection");
            SignalingError::Transport(anyhow::anyhow!(err))
        })
    }
}

#[async_trait]
impl SignalingReceiver for TokioReceiver {
    #[tracing::instrument(level = "debug", skip(self))]
    async fn recv(&mut self, send_tx: &mpsc::Sender<tungstenite::Message>) -> Result<SignalingMessage, SignalingError> {
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
                Ok(tungstenite::Message::Ping(data)) => {
                    if let Err(err) = send_tx
                        .send(tungstenite::Message::Pong(data))
                        .await
                    {
                        tracing::warn!(?err, "Failed to send tokio Pong");
                        return Err(SignalingError::Disconnected);
                    }
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
}