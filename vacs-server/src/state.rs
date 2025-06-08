use crate::config;
use crate::config::AppConfig;
use crate::ws::ClientSession;
use axum::extract::ws;
use futures_util::SinkExt;
use std::collections::HashMap;
use tokio::sync::{RwLock, broadcast, mpsc, watch};
use vacs_shared::signaling::{ClientInfo, Message};
use crate::ws::traits::WebSocketSink;

pub struct AppState {
    pub config: AppConfig,
    /// Key: CID
    clients: RwLock<HashMap<String, ClientSession>>,
    broadcast_tx: broadcast::Sender<Message>,
    shutdown_rx: watch::Receiver<()>,
}

impl AppState {
    pub fn new(config: AppConfig, shutdown_rx: watch::Receiver<()>) -> Self {
        let (broadcast_tx, _) = broadcast::channel(config::BROADCAST_CHANNEL_CAPACITY);
        Self {
            config,
            clients: RwLock::new(HashMap::new()),
            broadcast_tx,
            shutdown_rx,
        }
    }

    pub fn get_client_receivers(&self) -> (broadcast::Receiver<Message>, watch::Receiver<()>) {
        (self.broadcast_tx.subscribe(), self.shutdown_rx.clone())
    }

    pub async fn register_client(
        &self,
        client_id: &str,
    ) -> anyhow::Result<(ClientSession, mpsc::Receiver<Message>)> {
        tracing::trace!("Registering client");

        if self.clients.read().await.contains_key(client_id) {
            anyhow::bail!("Client already exists");
        }

        let (tx, rx) = mpsc::channel(config::CLIENT_CHANNEL_CAPACITY);
        let client = ClientSession::new(
            ClientInfo {
                id: client_id.to_string(),
                display_name: client_id.to_string(), // TODO retrieve actual display name
            },
            tx,
        );

        self.clients
            .write()
            .await
            .insert(client_id.to_string(), client.clone());

        if self.broadcast_tx.receiver_count() > 0 {
            tracing::trace!("Broadcasting client connected message");
            if let Err(err) = self.broadcast_tx.send(Message::ClientConnected {
                client: client.get_client_info().clone(),
            }) {
                tracing::warn!(?err, "Failed to broadcast client connected message");
            }
        } else {
            tracing::debug!(
                "No other broadcast receivers subscribed, skipping client connected message"
            );
        }

        tracing::trace!("Client registered");
        Ok((client, rx))
    }

    pub async fn unregister_client(&self, client_id: &str) {
        tracing::trace!("Unregistering client");

        if self.clients.write().await.remove(client_id).is_none() {
            tracing::debug!("Client not found in client list, skipping unregister");
            return;
        }

        if self.broadcast_tx.receiver_count() > 1 {
            tracing::trace!("Broadcasting client disconnected message");
            if let Err(err) = self.broadcast_tx.send(Message::ClientDisconnected {
                id: client_id.to_string(),
            }) {
                tracing::warn!(?err, "Failed to broadcast client disconnected message");
            }
        } else {
            tracing::debug!(
                "No other broadcast receivers subscribed, skipping client disconnected message"
            );
        }

        tracing::debug!("Client unregistered");
    }

    pub async fn list_clients(&self) -> Vec<ClientInfo> {
        self.clients
            .read()
            .await
            .values()
            .cloned()
            .map(|c| c.get_client_info().clone())
            .collect()
    }

    pub async fn get_client(&self, client_id: &str) -> Option<ClientSession> {
        self.clients.read().await.get(client_id).cloned()
    }

    pub async fn send_message_to_peer<T:WebSocketSink>(
        &self,
        client_websocket_tx: &mut T,
        peer_id: &str,
        message: Message,
    ) {
        match self.get_client(peer_id).await {
            Some(peer) => {
                tracing::trace!(peer_id, "Sending message to peer");
                if let Err(err) = peer.send_message(message).await {
                    tracing::warn!(?err, "Failed to send message to peer");
                }
            }
            None => {
                tracing::warn!(peer_id, "Peer not found");
                if let Err(err) = client_websocket_tx
                    .send(ws::Message::text(format!("Peer not found: {}", peer_id)))
                    .await
                {
                    tracing::warn!(
                        peer_id,
                        ?err,
                        "Failed to send peer not found reply to client"
                    );
                }
            }
        }
    }
}
