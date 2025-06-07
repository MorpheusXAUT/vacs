use axum::extract::ws;
use axum::extract::ws::WebSocket;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::SinkExt;
use futures_util::StreamExt;
use vacs_shared::signaling;

/// Represents the outcome of [`receive_message`], indicating whether the message received should be handled, skipped or receiving errored.
pub enum MessageResult {
    /// A valid application-message that can be processed.
    ApplicationMessage(signaling::Message),
    /// A control message (e.g., Ping, Pong) that should be skipped.
    ControlMessage,
    /// The client has disconnected.
    Disconnected,
    /// An error occurred while receiving the message.
    Error(anyhow::Error),
}

pub async fn send_message(
    websocket_sender: &mut SplitSink<WebSocket, ws::Message>,
    message: signaling::Message,
) -> anyhow::Result<()> {
    let serialized_message = signaling::Message::serialize(&message)
        .map_err(|e| anyhow::anyhow!(e).context("Failed to serialize message"))?;
    websocket_sender
        .send(ws::Message::from(serialized_message))
        .await
        .map_err(|e| anyhow::anyhow!(e).context("Failed to send message"))?;
    Ok(())
}

pub async fn receive_message(websocket_receiver: &mut SplitStream<WebSocket>) -> MessageResult {
    match websocket_receiver.next().await {
        Some(Ok(ws::Message::Text(raw_message))) => {
            match signaling::Message::deserialize(&raw_message) {
                Ok(message) => MessageResult::ApplicationMessage(message),
                Err(err) => MessageResult::Error(
                    anyhow::anyhow!(err).context("Failed to deserialize message"),
                ),
            }
        }
        Some(Ok(ws::Message::Ping(_))) => MessageResult::ControlMessage,
        Some(Ok(ws::Message::Pong(_))) => MessageResult::ControlMessage,
        Some(Ok(ws::Message::Close(reason))) => {
            tracing::debug!(?reason, "Received websocket close message");
            MessageResult::Disconnected
        }
        Some(Ok(other)) => {
            tracing::trace!(?other, "Received unexpected websocket message");
            MessageResult::Error(anyhow::anyhow!("Received unexpected websocket message"))
        }
        Some(Err(err)) => {
            tracing::warn!(?err, "Failed to receive message");
            MessageResult::Error(anyhow::anyhow!(err).context("Failed to receive message"))
        }
        None => {
            tracing::debug!("Client receiver closed, disconnecting");
            MessageResult::Disconnected
        }
    }
}
