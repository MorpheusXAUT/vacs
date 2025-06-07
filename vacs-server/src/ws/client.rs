use crate::state::AppState;
use crate::ws::application_message::handle_application_message;
use crate::ws::message::receive_message;
use crate::ws::message::{send_message, MessageResult};
use axum::extract::ws;
use axum::extract::ws::WebSocket;
use futures_util::stream::{SplitSink, SplitStream};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, watch};
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

    pub async fn handle_interaction(
        &mut self,
        app_state: &Arc<AppState>,
        websocket_rx: &mut SplitStream<WebSocket>,
        websocket_tx: &mut SplitSink<WebSocket, ws::Message>,
        broadcast_rx: &mut broadcast::Receiver<Message>,
        rx: &mut mpsc::Receiver<Message>,
        shutdown_rx: &mut watch::Receiver<()>,
    ) {
        tracing::debug!("Starting to handle client interaction");

        tracing::trace!("Sending initial client list");
        let clients = app_state.list_clients().await;
        if let Err(err) = send_message(websocket_tx, Message::ClientList { clients }).await {
            tracing::warn!(?err, "Failed to send initial client list");
        }

        loop {
            tokio::select! {
                biased;

                _ = shutdown_rx.changed() => {
                    tracing::trace!("Shutdown signal received, disconnecting client");
                    break;
                }

                message_result = receive_message(websocket_rx) => {
                    match message_result {
                        MessageResult::ApplicationMessage(message) => {
                            handle_application_message(app_state, self, websocket_tx, message).await;
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

                message = rx.recv() => {
                    match message {
                        Some(message) => {
                            if let Err(err) = send_message(websocket_tx, message).await {
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
                            if let Err(err) = send_message(websocket_tx, message).await {
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
