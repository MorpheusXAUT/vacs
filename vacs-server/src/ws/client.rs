use crate::config;
use crate::state::AppState;
use crate::ws::application_message::handle_application_message;
use crate::ws::message::{MessageResult, receive_message, send_message};
use crate::ws::traits::{WebSocketSink, WebSocketStream};
use std::ops::ControlFlow;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, watch};
use tracing::{instrument, Instrument};
use vacs_protocol::ws::{ClientInfo, SignalingMessage};

#[derive(Clone)]
pub struct ClientSession {
    client_info: ClientInfo,
    tx: mpsc::Sender<SignalingMessage>,
}

impl ClientSession {
    pub fn new(client_info: ClientInfo, tx: mpsc::Sender<SignalingMessage>) -> Self {
        Self { client_info, tx }
    }

    pub fn get_id(&self) -> &str {
        &self.client_info.id
    }

    pub fn get_client_info(&self) -> &ClientInfo {
        &self.client_info
    }

    pub async fn send_message(&self, message: SignalingMessage) -> anyhow::Result<()> {
        self.tx
            .send(message)
            .await
            .map_err(|err| anyhow::anyhow!(err).context("Failed to send message"))
    }

    #[allow(clippy::too_many_arguments)]
    #[instrument(level = "debug", skip_all, fields(client_id = %client_id))]
    pub async fn handle_interaction<R: WebSocketStream + 'static, T: WebSocketSink + 'static>(
        &mut self,
        app_state: &Arc<AppState>,
        mut websocket_rx: R,
        mut websocket_tx: T,
        broadcast_rx: &mut broadcast::Receiver<SignalingMessage>,
        rx: &mut mpsc::Receiver<SignalingMessage>,
        shutdown_rx: &mut watch::Receiver<()>,
        client_id: &str,
    ) {
        tracing::debug!("Starting to handle client interaction");

        tracing::trace!("Sending initial client list");
        let clients = app_state.list_clients_without_self(client_id).await;
        if let Err(err) =
            send_message(&mut websocket_tx, SignalingMessage::ClientList { clients }).await
        {
            tracing::warn!(?err, "Failed to send initial client list");
        }

        let (ws_tx, mut ws_rx) = mpsc::channel(config::CLIENT_WEBSOCKET_RECEIVE_CHANNEL_CAPACITY);
        let websocket_receive_task = tokio::spawn(
            async move {
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
                tracing::trace!("Finished receiving messages from client");
            }
            .instrument(tracing::Span::current()),
        );

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

        websocket_receive_task.abort();

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
    use test_log::test;

    #[test(tokio::test)]
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

    #[test(tokio::test)]
    async fn send_message() {
        let client_info = ClientInfo {
            id: "client1".to_string(),
            display_name: "Client 1".to_string(),
        };
        let (tx, mut rx) = mpsc::channel(10);
        let session = ClientSession::new(client_info, tx);

        let message = SignalingMessage::ClientList {
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

    #[test(tokio::test)]
    async fn send_message_error() {
        let client_info = ClientInfo {
            id: "client1".to_string(),
            display_name: "Client 1".to_string(),
        };
        let (tx, _) = mpsc::channel(10);
        let session = ClientSession::new(client_info, tx.clone());
        drop(tx); // Drop the sender to simulate the client disconnecting

        let message = SignalingMessage::ClientList {
            clients: vec![ClientInfo {
                id: "client2".to_string(),
                display_name: "Client 2".to_string(),
            }],
        };
        let result = session.send_message(message.clone()).await;

        assert!(result.is_err_and(|err| err.to_string().contains("Failed to send message")));
    }

    #[test(tokio::test)]
    async fn initial_client_list_without_self() {
        let setup = TestSetup::new();
        setup.register_client("client1").await;
        let websocket_rx = setup.websocket_rx.clone();

        let handle_task = setup.spawn_session_handle_interaction("client1".to_string());

        let message = websocket_rx.lock().unwrap().recv().await;
        match message {
            Some(ws::Message::Text(text)) => {
                assert_eq!(
                    text,
                    Utf8Bytes::from_static(
                        r#"{"ClientList":{"clients":[]}}"#
                    )
                );
            }
            _ => panic!("Expected client list message"),
        }

        handle_task.await.unwrap();
    }

    #[test(tokio::test)]
    async fn initial_client_list() {
        let setup = TestSetup::new();
        setup.register_client("client1").await;
        setup.register_client("client2").await;
        let websocket_rx = setup.websocket_rx.clone();

        let handle_task = setup.spawn_session_handle_interaction("client2".to_string());

        let message = websocket_rx.lock().unwrap().recv().await;
        match message {
            Some(ws::Message::Text(text)) => {
                assert_eq!(
                    text,
                    Utf8Bytes::from_static(
                        r#"{"ClientList":{"clients":[{"id":"client1","displayName":"client1"}]}}"#
                    )
                );
            }
            _ => panic!("Expected client list message"),
        }

        handle_task.await.unwrap();
    }

    #[test(tokio::test)]
    async fn handle_interaction() {
        let setup = TestSetup::new().with_messages(vec![Ok(ws::Message::Text(
            Utf8Bytes::from_static(r#"{"CallOffer":{"peer_id":"client2","sdp":"sdp1"}}"#),
        ))]);
        let (_, mut client2_rx) = setup.register_client("client2").await;
        let websocket_rx = setup.websocket_rx.clone();

        let handle_task = setup.spawn_session_handle_interaction("client1".to_string());

        let message = websocket_rx.lock().unwrap().recv().await;
        match message {
            Some(ws::Message::Text(text)) => {
                assert_eq!(
                    text,
                    Utf8Bytes::from_static(
                        r#"{"ClientList":{"clients":[{"id":"client2","displayName":"client2"}]}}"#
                    )
                );
            }
            _ => panic!("Expected client list message"),
        }

        let call_offer = client2_rx.recv().await.unwrap();
        assert_eq!(
            call_offer,
            SignalingMessage::CallOffer {
                peer_id: "client1".to_string(),
                sdp: "sdp1".to_string()
            }
        );

        handle_task.await.unwrap();
    }

    #[test(tokio::test)]
    async fn handle_interaction_websocket_error() {
        let setup = TestSetup::new().with_messages(vec![Err(axum::Error::new("Test error"))]);
        let websocket_rx = setup.websocket_rx.clone();

        let handle_task = setup.spawn_session_handle_interaction("client1".to_string());

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
