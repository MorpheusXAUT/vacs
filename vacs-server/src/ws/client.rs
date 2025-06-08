use crate::state::AppState;
use crate::ws::application_message::handle_application_message;
use crate::ws::message::{receive_message, send_message, MessageResult};
use crate::ws::traits::{WebSocketSink, WebSocketStream};
use std::ops::ControlFlow;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, watch};
use tracing::Instrument;
use vacs_shared::signaling;
use vacs_shared::signaling::Message;

#[derive(Clone)]
pub struct ClientSession {
    client_info: signaling::ClientInfo,
    tx: mpsc::Sender<Message>,
}

impl ClientSession {
    pub fn new(client_info: signaling::ClientInfo, tx: mpsc::Sender<Message>) -> Self {
        Self { client_info, tx }
    }

    pub fn get_id(&self) -> &str {
        &self.client_info.id
    }

    pub fn get_client_info(&self) -> &signaling::ClientInfo {
        &self.client_info
    }

    pub async fn send_message(&self, message: Message) -> anyhow::Result<()> {
        self.tx
            .send(message)
            .await
            .map_err(|err| anyhow::anyhow!(err).context("Failed to send message"))
    }

    pub async fn handle_interaction<R: WebSocketStream + 'static, T: WebSocketSink + 'static>(
        &mut self,
        app_state: &Arc<AppState>,
        mut websocket_rx: R,
        mut websocket_tx: T,
        broadcast_rx: &mut broadcast::Receiver<Message>,
        rx: &mut mpsc::Receiver<Message>,
        shutdown_rx: &mut watch::Receiver<()>,
    ) {
        tracing::debug!("Starting to handle client interaction");

        tracing::trace!("Sending initial client list");
        let clients = app_state.list_clients().await;
        if let Err(err) = send_message(&mut websocket_tx, Message::ClientList { clients }).await {
            tracing::warn!(?err, "Failed to send initial client list");
        }

        let (ws_tx, mut ws_rx) = mpsc::channel(10);
        tokio::spawn(async move {
            loop {
                let message_result = receive_message(&mut websocket_rx).await;
                match message_result {
                    MessageResult::ApplicationMessage(message) => {
                        tracing::trace!("Forwarding message to application");
                        if let Err(err) = ws_tx.send(message).await {
                            tracing::warn!(?err, "Failed to forward message to application");
                        }
                    }
                    MessageResult::ControlMessage => continue,
                    MessageResult::Disconnected => {
                        tracing::debug!("Client disconnected");
                        break;
                    }
                    MessageResult::Error(err) => {
                        tracing::warn!(?err, "Error while receiving message from client");
                        break;
                    }
                }
            }
        }.instrument(tracing::Span::current()));

        loop {
            tokio::select! {
                biased;

                _ = shutdown_rx.changed() => {
                    tracing::trace!("Shutdown signal received, disconnecting client");
                    break;
                }

                message = ws_rx.recv() => {
                    match message {
                        Some(message) => {
                            match handle_application_message(app_state, self, &mut websocket_tx, message).await {
                                ControlFlow::Continue(()) => continue,
                                ControlFlow::Break(()) => {
                                    tracing::debug!("Breaking interaction loop");
                                    break;
                                },
                            }
                        }
                        None => {
                            tracing::debug!("Application receiver closed, disconnecting client");
                            break;
                        }
                    }
                }

                message = rx.recv() => {
                    match message {
                        Some(message) => {
                            tracing::trace!("Received direct message");
                            if let Err(err) = send_message(&mut websocket_tx, message).await {
                                tracing::warn!(?err, "Failed to send direct message");
                            }
                        }
                        None => {
                            tracing::debug!("Client receiver closed, disconnecting client");
                            break;
                        }
                    }
                }

                message = broadcast_rx.recv() => {
                    match message {
                        Ok(message) => {
                            tracing::trace!("Received broadcast message");
                            if let Err(err) = send_message(&mut websocket_tx, message).await {
                                tracing::warn!(?err, "Failed to send broadcast message");
                            }
                        }
                        Err(err) => {
                            tracing::debug!(?err, "Broadcast receiver closed, disconnecting client");
                        }
                    }
                }
            }
        }

        tracing::debug!("Finished handling client interaction");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws::test_util::TestSetup;
    use axum::extract::ws;
    use axum::extract::ws::Utf8Bytes;
    use pretty_assertions::assert_eq;
    use vacs_shared::signaling::ClientInfo;

    #[tokio::test]
    async fn new_client_session() {
        let client_info = ClientInfo {
            id: "client1".to_string(),
            display_name: "Client 1".to_string(),
        };
        let (tx, _rx) = mpsc::channel(10);
        let session = ClientSession::new(client_info.clone(), tx);

        assert_eq!(session.get_id(), "client1");
        assert_eq!(session.get_client_info(), &client_info);
    }

    #[tokio::test]
    async fn send_message() {
        let client_info = ClientInfo {
            id: "client1".to_string(),
            display_name: "Client 1".to_string(),
        };
        let (tx, mut rx) = mpsc::channel(10);
        let session = ClientSession::new(client_info, tx);

        let message = Message::ClientList {
            clients: vec![ClientInfo {
                id: "client2".to_string(),
                display_name: "Client 2".to_string(),
            }],
        };
        let result = session.send_message(message.clone()).await;

        assert!(result.is_ok());
        let received = rx.recv().await.expect("Expected message to be received");
        assert_eq!(received, message);
    }

    #[tokio::test]
    async fn send_message_error() {
        let client_info = ClientInfo {
            id: "client1".to_string(),
            display_name: "Client 1".to_string(),
        };
        let (tx, _) = mpsc::channel(10);
        let session = ClientSession::new(client_info, tx.clone());
        drop(tx); // Drop the sender to simulate the client disconnecting

        let message = Message::ClientList {
            clients: vec![ClientInfo {
                id: "client2".to_string(),
                display_name: "Client 2".to_string(),
            }],
        };
        let result = session.send_message(message.clone()).await;

        assert!(result.is_err_and(|err| err.to_string().contains("Failed to send message")));
    }

    #[tokio::test]
    async fn initial_client_list() {
        let setup = TestSetup::new();
        setup.register_client("client1").await;
        let websocket_rx = setup.websocket_rx.clone();

        let handle_task = setup.spawn_session_handle_interaction();

        let message = websocket_rx.lock().unwrap().recv().await;
        match message {
            Some(ws::Message::Text(text)) => {
                assert_eq!(
                    text,
                    Utf8Bytes::from_static(
                        r#"{"ClientList":{"clients":[{"id":"client1","display_name":"client1"}]}}"#
                    )
                );
            }
            _ => panic!("Expected client list message"),
        }

        handle_task.await.unwrap();
    }

    #[tokio::test]
    async fn handle_interaction() {
        let setup = TestSetup::new().with_messages(vec![Ok(ws::Message::Text(
            Utf8Bytes::from_static(r#"{"CallOffer":{"peer_id":"client2","sdp":"sdp1"}}"#),
        ))]);
        let (_, mut client2_rx) = setup.register_client("client2").await;
        let websocket_rx = setup.websocket_rx.clone();

        let handle_task = setup.spawn_session_handle_interaction();

        let message = websocket_rx.lock().unwrap().recv().await;
        match message {
            Some(ws::Message::Text(text)) => {
                assert_eq!(
                    text,
                    Utf8Bytes::from_static(
                        r#"{"ClientList":{"clients":[{"id":"client2","display_name":"client2"}]}}"#
                    )
                );
            }
            _ => panic!("Expected client list message"),
        }

        let call_offer = client2_rx.recv().await.unwrap();
        assert_eq!(
            call_offer,
            Message::CallOffer {
                peer_id: "client1".to_string(),
                sdp: "sdp1".to_string()
            }
        );

        handle_task.await.unwrap();
    }

    #[tokio::test]
    async fn handle_interaction_websocket_error() {
        let setup = TestSetup::new().with_messages(vec![Err(axum::Error::new("Test error"))]);
        let websocket_rx = setup.websocket_rx.clone();

        let handle_task = setup.spawn_session_handle_interaction();

        let message = websocket_rx.lock().unwrap().recv().await;
        match message {
            Some(ws::Message::Text(text)) => {
                assert_eq!(
                    text,
                    Utf8Bytes::from_static(r#"{"ClientList":{"clients":[]}}"#)
                );
            }
            _ => panic!("Expected client list message"),
        }

        assert!(websocket_rx.lock().unwrap().is_closed());

        handle_task.await.unwrap();
    }
}
