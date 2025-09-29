use crate::config;
use crate::config::AppConfig;
use crate::release::UpdateChecker;
use crate::store::{Store, StoreBackend};
use crate::ws::ClientSession;
use anyhow::Context;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast, mpsc, watch};
use tracing::instrument;
use uuid::Uuid;
use vacs_protocol::ws::{ClientInfo, ErrorReason, SignalingMessage};
use vacs_vatsim::slurper::{SlurperClient, SlurperUserInfo};

pub struct AppState {
    pub config: AppConfig,
    pub updates: UpdateChecker,
    store: Store,
    /// Key: CID
    clients: RwLock<HashMap<String, ClientSession>>,
    broadcast_tx: broadcast::Sender<SignalingMessage>,
    slurper: SlurperClient,
    shutdown_rx: watch::Receiver<()>,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        updates: UpdateChecker,
        store: Store,
        slurper_client: SlurperClient,
        shutdown_rx: watch::Receiver<()>,
    ) -> Self {
        let (broadcast_tx, _) = broadcast::channel(config::BROADCAST_CHANNEL_CAPACITY);
        Self {
            config,
            updates,
            store,
            clients: RwLock::new(HashMap::new()),
            slurper: slurper_client,
            broadcast_tx,
            shutdown_rx,
        }
    }

    pub fn get_client_receivers(
        &self,
    ) -> (broadcast::Receiver<SignalingMessage>, watch::Receiver<()>) {
        (self.broadcast_tx.subscribe(), self.shutdown_rx.clone())
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn register_client(
        &self,
        client_info: ClientInfo,
    ) -> anyhow::Result<(ClientSession, mpsc::Receiver<SignalingMessage>)> {
        tracing::trace!("Registering client");

        let client_id = client_info.id.clone();
        if self.clients.read().await.contains_key(&client_id) {
            tracing::trace!("Client already exists");
            anyhow::bail!("Client already exists");
        }

        let (tx, rx) = mpsc::channel(config::CLIENT_CHANNEL_CAPACITY);
        let client = ClientSession::new(client_info, tx);

        self.clients
            .write()
            .await
            .insert(client_id.to_string(), client.clone());

        if self.broadcast_tx.receiver_count() > 0 {
            tracing::trace!("Broadcasting client connected message");
            if let Err(err) = self.broadcast_tx.send(SignalingMessage::ClientConnected {
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

    #[instrument(level = "debug", skip(self))]
    pub async fn unregister_client(&self, client_id: &str) {
        tracing::trace!("Unregistering client");

        // TODO notify client about termination to avoid reconnect loop
        let Some(client) = self.clients.write().await.remove(client_id) else {
            tracing::debug!("Client not found in client list, skipping unregister");
            return;
        };
        client.disconnect();

        if self.broadcast_tx.receiver_count() > 1 {
            tracing::trace!("Broadcasting client disconnected message");
            if let Err(err) = self
                .broadcast_tx
                .send(SignalingMessage::ClientDisconnected {
                    id: client_id.to_string(),
                })
            {
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
        let mut clients: Vec<ClientInfo> = self
            .clients
            .read()
            .await
            .values()
            .cloned()
            .map(|c| c.get_client_info().clone())
            .collect();

        clients.sort_by(|a, b| a.id.cmp(&b.id));
        clients
    }

    pub async fn list_clients_without_self(&self, self_client_id: &str) -> Vec<ClientInfo> {
        self.list_clients()
            .await
            .into_iter()
            .filter(|c| c.id != self_client_id)
            .collect()
    }

    pub async fn get_client(&self, client_id: &str) -> Option<ClientSession> {
        self.clients.read().await.get(client_id).cloned()
    }

    pub async fn send_message_to_peer(
        &self,
        client: &ClientSession,
        peer_id: &str,
        message: SignalingMessage,
    ) {
        match self.get_client(peer_id).await {
            Some(peer) => {
                tracing::trace!(?peer_id, "Sending message to peer");
                if let Err(err) = peer.send_message(message).await {
                    tracing::warn!(?err, "Failed to send message to peer");
                    if let Err(e) = client
                        .send_message(SignalingMessage::Error {
                            reason: ErrorReason::PeerConnection,
                            peer_id: Some(peer_id.to_string()),
                        })
                        .await
                    {
                        tracing::warn!(?peer_id, orig_err = ?err, err = ?e, "Failed to send error message to client");
                    }
                }
            }
            None => {
                tracing::warn!(peer_id, "Peer not found");
                if let Err(err) = client
                    .send_message(SignalingMessage::PeerNotFound {
                        peer_id: peer_id.to_string(),
                    })
                    .await
                {
                    tracing::warn!(
                        ?peer_id,
                        ?err,
                        "Failed to send peer not found message to client"
                    );
                }
            }
        }
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn generate_ws_auth_token(&self, cid: &str) -> anyhow::Result<String> {
        tracing::debug!("Generating web socket auth token");

        let token = Uuid::new_v4().to_string();

        tracing::trace!("Storing web socket auth token");
        self.store
            .set(
                format!("ws.token.{token}").as_str(),
                cid,
                Some(Duration::from_secs(30)),
            )
            .await
            .context("Failed to store web socket auth token")?;

        tracing::debug!("Web socket auth token generated");
        Ok(token)
    }

    #[instrument(level = "debug", skip_all, err)]
    pub async fn verify_ws_auth_token(&self, token: &str) -> anyhow::Result<String> {
        tracing::debug!("Verifying web socket auth token");

        match self.store.get(format!("ws.token.{token}").as_str()).await {
            Ok(Some(cid)) => {
                tracing::debug!(?cid, "Web socket auth token verified");
                Ok(cid)
            }
            Ok(None) => anyhow::bail!("Web socket auth token not found"),
            Err(err) => anyhow::bail!(err),
        }
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn get_vatsim_user_info(&self, cid: &str) -> anyhow::Result<Option<SlurperUserInfo>> {
        tracing::debug!("Retrieving connection info from VATSIM slurper");
        self.slurper.get_user_info(cid).await
    }

    pub async fn health_check(&self) -> anyhow::Result<()> {
        self.store.is_healthy().await
    }
}
