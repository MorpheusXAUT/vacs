use axum::extract::ws;
use futures_util::{Sink, SinkExt};
use futures_util::{Stream, StreamExt};
use vacs_shared::signaling;

/// Represents the outcome of [`receive_message`], indicating whether the message received should be handled, skipped or receiving errored.
#[derive(Debug)]
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

impl PartialEq for MessageResult {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MessageResult::ApplicationMessage(a), MessageResult::ApplicationMessage(b)) => a == b,
            (MessageResult::ControlMessage, MessageResult::ControlMessage) => true,
            (MessageResult::Disconnected, MessageResult::Disconnected) => true,
            (MessageResult::Error(self_err), MessageResult::Error(other_err)) => {
                self_err.to_string() == other_err.to_string()
            }
            _ => false,
        }
    }
}

pub async fn send_message<S>(
    websocket_tx: &mut S,
    message: signaling::Message,
) -> anyhow::Result<()>
where
    S: Sink<ws::Message> + Unpin,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    let serialized_message = signaling::Message::serialize(&message)
        .map_err(|e| anyhow::anyhow!(e).context("Failed to serialize message"))?;
    websocket_tx
        .send(ws::Message::from(serialized_message))
        .await
        .map_err(|e| anyhow::anyhow!(e).context("Failed to send message"))?;
    Ok(())
}

pub async fn receive_message<S>(websocket_rx: &mut S) -> MessageResult
where
    S: Stream<Item = Result<ws::Message, axum::Error>> + Unpin,
{
    match websocket_rx.next().await {
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{Sink, Stream};
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll};
    use tokio::sync::{mpsc, Mutex};
    use tokio_tungstenite::tungstenite;

    #[derive(Debug)]
    struct MockSinkError(String);

    impl std::fmt::Display for MockSinkError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "MockSinkError: {}", self.0)
        }
    }

    impl std::error::Error for MockSinkError {}

    struct MockSink {
        tx: mpsc::UnboundedSender<ws::Message>,
    }

    impl Sink<ws::Message> for MockSink {
        type Error = MockSinkError;

        fn poll_ready(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn start_send(self: Pin<&mut Self>, item: ws::Message) -> Result<(), Self::Error> {
            self.tx.send(item).map_err(|e| MockSinkError(e.to_string()))
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    struct MockStream {
        messages: Vec<Result<ws::Message, axum::Error>>,
    }

    impl Stream for MockStream {
        type Item = Result<ws::Message, axum::Error>;

        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            if self.messages.is_empty() {
                Poll::Ready(None)
            } else {
                Poll::Ready(Some(self.messages.remove(0)))
            }
        }
    }

    #[tokio::test]
    async fn send_single_message() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut mock_sink = MockSink { tx };

        let message = signaling::Message::ClientConnected {
            client: signaling::ClientInfo {
                id: "client1".to_string(),
                display_name: "Client 1".to_string(),
            },
        };

        assert!(send_message(&mut mock_sink, message.clone()).await.is_ok());

        if let Some(sent_message) = rx.recv().await {
            if let ws::Message::Text(serialized_message) = sent_message {
                let deserialized_message = signaling::Message::deserialize(&serialized_message)
                    .expect("Failed to deserialize message");
                assert_eq!(deserialized_message, message);
            } else {
                panic!("Expected a Text message, got: {:?}", sent_message);
            }
        } else {
            panic!("No message received");
        }
    }

    #[tokio::test]
    async fn send_multiple_messages() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut mock_sink = MockSink { tx };

        let messages = vec![
            signaling::Message::Login {
                id: "client1".to_string(),
                token: "token1".to_string(),
            },
            signaling::Message::ListClients,
            signaling::Message::Logout,
        ];
        for message in &messages {
            assert!(send_message(&mut mock_sink, message.clone()).await.is_ok());
        }

        for expected in messages {
            let sent = rx.recv().await.expect("No message received");
            match sent {
                ws::Message::Text(raw_message) => {
                    let message = signaling::Message::deserialize(&raw_message)
                        .expect("Failed to deserialize message");
                    assert_eq!(message, expected);
                }
                _ => panic!("Expected a Text message, got: {:?}", sent),
            }
        }
    }

    #[tokio::test]
    async fn send_messages_concurrently() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mock_sink = Arc::new(Mutex::new(MockSink { tx }));

        let messages = vec![
            signaling::Message::Login {
                id: "client1".to_string(),
                token: "token1".to_string(),
            },
            signaling::Message::ListClients,
            signaling::Message::Logout,
        ];

        let mut tasks = vec![];
        for message in &messages {
            let mock_sink = mock_sink.clone();
            let message = message.clone();
            let task = tokio::spawn(async move {
                let mut mock_sink = mock_sink.lock().await;
                send_message(&mut *mock_sink, message.clone()).await
            });
            tasks.push(task);
        }

        let results = futures_util::future::join_all(tasks).await;

        for result in results {
            assert!(result.unwrap().is_ok(), "Sending message failed");
        }

        let mut sent = vec![];
        for _ in 0..messages.len() {
            let msg = rx.recv().await.expect("Expected a message");
            if let ws::Message::Text(raw_message) = msg {
                let message = signaling::Message::deserialize(&raw_message)
                    .expect("Failed to deserialize message");
                sent.push(message);
            }
        }

        for expected in &messages {
            assert!(messages.contains(expected));
        }
        assert_eq!(sent.len(), messages.len());
    }

    #[tokio::test]
    async fn send_message_sink_disconnected() {
        let (tx, rx) = mpsc::unbounded_channel();
        drop(rx); // Drop the receiver to simulate the sink being disconnected.
        let mut mock_sink = MockSink { tx };

        let message = signaling::Message::ClientConnected {
            client: signaling::ClientInfo {
                id: "client1".to_string(),
                display_name: "Client 1".to_string(),
            },
        };

        assert!(
            send_message(&mut mock_sink, message.clone())
                .await
                .is_err_and(|err| err.to_string().contains("Failed to send message"))
        );
    }

    #[tokio::test]
    async fn receive_single_message() {
        let mut mock_stream = MockStream {
            messages: vec![Ok(ws::Message::from(
                "{\"Login\":{\"id\":\"client1\",\"token\":\"token1\"}}",
            ))],
        };

        let result = receive_message(&mut mock_stream).await;

        assert_eq!(
            result,
            MessageResult::ApplicationMessage(signaling::Message::Login {
                id: "client1".to_string(),
                token: "token1".to_string()
            })
        );
    }

    #[tokio::test]
    async fn receive_multiple_messages() {
        let mut mock_stream = MockStream {
            messages: vec![
                Ok(ws::Message::from(
                    "{\"Login\":{\"id\":\"client1\",\"token\":\"token1\"}}",
                )),
                Ok(ws::Message::from("\"Logout\"")),
                Ok(ws::Message::from(
                    "{\"CallOffer\":{\"peer_id\":\"client1\",\"sdp\":\"sdp1\"}}",
                )),
            ],
        };

        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::ApplicationMessage(signaling::Message::Login {
                id: "client1".to_string(),
                token: "token1".to_string()
            })
        );
        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::ApplicationMessage(signaling::Message::Logout)
        );
        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::ApplicationMessage(signaling::Message::CallOffer {
                peer_id: "client1".to_string(),
                sdp: "sdp1".to_string()
            })
        );
    }

    #[tokio::test]
    async fn receive_messages_concurrently() {
        let mock_stream = Arc::new(Mutex::new(MockStream {
            messages: vec![
                Ok(ws::Message::from(
                    "{\"Login\":{\"id\":\"client1\",\"token\":\"token1\"}}",
                )),
                Ok(ws::Message::from("\"Logout\"")),
                Ok(ws::Message::from(
                    "{\"CallOffer\":{\"peer_id\":\"client1\",\"sdp\":\"sdp1\"}}",
                )),
            ],
        }));

        let mut tasks = vec![];
        for _ in 0..3 {
            let mock_stream = mock_stream.clone();
            let task = tokio::spawn(async move {
                let mut mock_stream = mock_stream.lock().await;
                receive_message(&mut *mock_stream).await
            });
            tasks.push(task);
        }

        let results = futures_util::future::join_all(tasks).await;
        for result in results {
            assert!(result.is_ok(), "Receiving message failed");
            assert!(matches!(
                result.unwrap(),
                MessageResult::ApplicationMessage(_)
            ));
        }
    }

    #[tokio::test]
    async fn receive_replayed_messages() {
        let msg = ws::Message::from("{\"Login\":{\"id\":\"client1\",\"token\":\"token1\"}}");
        let mut mock_stream = MockStream {
            messages: vec![Ok(msg.clone()), Ok(msg)],
        };

        for _ in 0..2 {
            assert_eq!(
                receive_message(&mut mock_stream).await,
                MessageResult::ApplicationMessage(signaling::Message::Login {
                    id: "client1".to_string(),
                    token: "token1".to_string()
                })
            );
        }
    }

    #[tokio::test]
    async fn receive_control_messages() {
        let mut mock_stream = MockStream {
            messages: vec![
                Ok(ws::Message::Ping(tungstenite::Bytes::from("ping"))),
                Ok(ws::Message::Pong(tungstenite::Bytes::from("pong"))),
            ],
        };

        for _ in 0..2 {
            assert_eq!(
                receive_message(&mut mock_stream).await,
                MessageResult::ControlMessage
            );
        }
    }

    #[tokio::test]
    async fn receive_close_message() {
        let mut mock_stream = MockStream {
            messages: vec![Ok(ws::Message::Close(None))],
        };

        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::Disconnected
        );
    }

    #[tokio::test]
    async fn receive_close_message_with_close_frame() {
        let mut mock_stream = MockStream {
            messages: vec![Ok(ws::Message::Close(Some(ws::CloseFrame {
                reason: ws::Utf8Bytes::from("goodbye"),
                code: 69,
            })))],
        };

        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::Disconnected
        );
    }

    #[tokio::test]
    async fn receive_mixed_messages() {
        let mut mock_stream = MockStream {
            messages: vec![
                Ok(ws::Message::Ping(tungstenite::Bytes::from("ping"))),
                Ok(ws::Message::from("\"Logout\"")),
                Ok(ws::Message::Pong(tungstenite::Bytes::from("pong"))),
            ],
        };

        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::ControlMessage
        );
        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::ApplicationMessage(signaling::Message::Logout)
        );
        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::ControlMessage
        );
    }

    #[tokio::test]
    async fn receive_message_deserialization_error() {
        let mut mock_stream = MockStream {
            messages: vec![Ok(ws::Message::Text(ws::Utf8Bytes::from("invalid")))],
        };

        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::Error(anyhow::anyhow!("Failed to deserialize message"))
        );
    }

    #[tokio::test]
    async fn receive_message_invalid_json() {
        let mut mock_stream = MockStream {
            messages: vec![Ok(ws::Message::Text(ws::Utf8Bytes::from("\"Logout")))],
        };

        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::Error(anyhow::anyhow!("Failed to deserialize message"))
        );
    }

    #[tokio::test]
    async fn receive_unknown_message_type() {
        let mut mock_stream = MockStream {
            messages: vec![Ok(ws::Message::Text(ws::Utf8Bytes::from(
                "{\"InvalidMessageType\":{\"unknown_field\":\"value\"}}",
            )))],
        };

        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::Error(anyhow::anyhow!("Failed to deserialize message"))
        );
    }

    #[tokio::test]
    async fn receive_empty_text() {
        let mut mock_stream = MockStream {
            messages: vec![Ok(ws::Message::Text(ws::Utf8Bytes::from("")))],
        };

        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::Error(anyhow::anyhow!("Failed to deserialize message"))
        );
    }

    #[tokio::test]
    async fn receive_message_abrupt_disconnect() {
        let mut mock_stream = MockStream {
            messages: vec![Err(axum::Error::new(tungstenite::Error::Io(
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Abrupt disconnection"),
            )))],
        };

        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::Error(anyhow::anyhow!("Failed to receive message"))
        );
    }

    #[tokio::test]
    async fn receive_unexpected_message() {
        let mut mock_stream = MockStream {
            messages: vec![Ok(ws::Message::Binary(tungstenite::Bytes::from("binary")))],
        };

        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::Error(anyhow::anyhow!("Received unexpected websocket message"))
        );
    }

    #[tokio::test]
    async fn receive_message_socket_error() {
        let mut mock_stream = MockStream {
            messages: vec![Err(axum::Error::new(tungstenite::Error::ConnectionClosed))],
        };

        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::Error(anyhow::anyhow!("Failed to receive message"))
        );
    }

    #[tokio::test]
    async fn receive_message_stream_end() {
        let mut mock_stream = MockStream { messages: vec![] };

        assert_eq!(
            receive_message(&mut mock_stream).await,
            MessageResult::Disconnected
        );
    }
}
