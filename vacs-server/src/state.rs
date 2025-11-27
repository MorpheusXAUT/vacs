use crate::config;
use crate::config::AppConfig;
use crate::metrics::ErrorMetrics;
use crate::ratelimit::RateLimiters;
use crate::release::UpdateChecker;
use crate::store::{Store, StoreBackend};
use crate::ws::ClientSession;
use anyhow::Context;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast, mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{Instrument, instrument};
use uuid::Uuid;
use vacs_protocol::ws::{ClientInfo, DisconnectReason, ErrorReason, SignalingMessage};
use vacs_vatsim::data_feed::DataFeed;
use vacs_vatsim::slurper::SlurperClient;
use vacs_vatsim::{ControllerInfo, FacilityType};

pub struct AppState {
    pub config: AppConfig,
    pub updates: UpdateChecker,
    store: Store,
    /// Key: CID
    clients: RwLock<HashMap<String, ClientSession>>,
    broadcast_tx: broadcast::Sender<SignalingMessage>,
    slurper: SlurperClient,
    data_feed: Arc<dyn DataFeed>,
    rate_limiters: RateLimiters,
    shutdown_rx: watch::Receiver<()>,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        updates: UpdateChecker,
        store: Store,
        slurper: SlurperClient,
        data_feed: Arc<dyn DataFeed>,
        rate_limiters: RateLimiters,
        shutdown_rx: watch::Receiver<()>,
    ) -> Self {
        let (broadcast_tx, _) = broadcast::channel(config::BROADCAST_CHANNEL_CAPACITY);
        Self {
            config,
            updates,
            store,
            clients: RwLock::new(HashMap::new()),
            broadcast_tx,
            slurper,
            data_feed,
            rate_limiters,
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
                client: client.client_info.clone(),
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
    pub async fn unregister_client(
        &self,
        client_id: &str,
        disconnect_reason: Option<DisconnectReason>,
    ) {
        tracing::trace!("Unregistering client");

        let Some(client) = self.clients.write().await.remove(client_id) else {
            tracing::debug!("Client not found in client list, skipping unregister");
            return;
        };

        client.disconnect(disconnect_reason);

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
            .map(|c| c.client_info.clone())
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
                    ErrorMetrics::error(&ErrorReason::PeerConnection);
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
                ErrorMetrics::peer_not_found();
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
    pub async fn get_vatsim_controller_info(
        &self,
        cid: &str,
    ) -> anyhow::Result<Option<ControllerInfo>> {
        tracing::debug!("Retrieving connection info from VATSIM slurper");
        self.slurper.get_controller_info(cid).await
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn get_vatsim_controllers(&self) -> anyhow::Result<Vec<ControllerInfo>> {
        tracing::debug!("Retrieving controller info from VATSIM data feed");
        self.data_feed.fetch_controller_info().await
    }

    #[instrument(level = "debug", skip(state))]
    pub fn start_controller_update_task(
        state: Arc<AppState>,
        interval: Duration,
    ) -> JoinHandle<()> {
        tokio::spawn(
            async move {
                let mut ticker = time::interval(interval);
                ticker.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

                let mut shutdown = state.shutdown_rx.clone();
                let mut pending_disconnect = HashSet::new();
                loop {
                    tokio::select! {
                        biased;
                        _ = shutdown.changed() => {
                            tracing::info!("Shutting down controller update task");
                            break;
                        }
                        _ = ticker.tick() => {
                            if state.clients.read().await.is_empty() {
                                tracing::trace!("No clients connected, skipping controller update");
                                continue;
                            }

                            tracing::debug!("Updating controller info");
                            if let Err(err) = Self::update_vatsim_controllers(&state, &mut pending_disconnect).await {
                                tracing::warn!(?err, "Failed to update controller info");
                            }
                        }
                    }
                }
            }
            .in_current_span(),
        )
    }

    async fn update_vatsim_controllers(
        state: &Arc<AppState>,
        pending_disconnect: &mut HashSet<String>,
    ) -> anyhow::Result<()> {
        let controllers = state.get_vatsim_controllers().await?;
        let current: HashMap<String, ControllerInfo> = controllers
            .into_iter()
            .map(|c| (c.cid.clone(), c))
            .collect();

        let mut updates: Vec<SignalingMessage> = Vec::new();
        let mut disconnected_clients: Vec<String> = Vec::new();

        fn flag_or_disconnect_controller(
            cid: &str,
            pending_disconnect: &mut HashSet<String>,
            disconnected_clients: &mut Vec<String>,
        ) {
            if pending_disconnect.remove(cid) {
                tracing::trace!(
                    ?cid,
                    "No active VATSIM connection found after grace period, disconnecting client and sending broadcast"
                );
                disconnected_clients.push(cid.to_string());
            } else {
                tracing::trace!(
                    ?cid,
                    "Client not found in data feed, but active VATSIM connection is required, marking for disconnect"
                );
                pending_disconnect.insert(cid.to_string());
            }
        }

        {
            let mut clients = state.clients.write().await;
            for (cid, session) in clients.iter_mut() {
                tracing::trace!(?cid, ?session, "Checking session for client info update");

                match current.get(cid) {
                    Some(controller) if controller.facility_type == FacilityType::Unknown => {
                        flag_or_disconnect_controller(
                            cid,
                            pending_disconnect,
                            &mut disconnected_clients,
                        )
                    }
                    Some(controller) => {
                        if pending_disconnect.remove(cid) {
                            tracing::trace!(
                                ?cid,
                                "Found active VATSIM connection for client again, removing pending disconnect"
                            );
                        }

                        let mut changed = false;
                        if session.client_info.display_name != controller.callsign {
                            tracing::trace!(
                                ?cid,
                                old = ?session.client_info.display_name,
                                new = ?controller.callsign,
                                "Controller display name changed, updating"
                            );
                            session.client_info.display_name = controller.callsign.clone();
                            changed = true;
                        }
                        if session.client_info.frequency != controller.frequency {
                            tracing::trace!(
                                ?cid,
                                old = ?session.client_info.frequency,
                                new = ?controller.frequency,
                                "Controller frequency changed, updating"
                            );
                            session.client_info.frequency = controller.frequency.clone();
                            changed = true;
                        }

                        if changed {
                            tracing::trace!(?cid, ?session, "Client info updated, broadcasting");
                            updates.push(SignalingMessage::ClientInfo {
                                own: false,
                                info: session.client_info.clone(),
                            });
                        } else {
                            tracing::trace!(
                                ?cid,
                                ?session,
                                ?controller,
                                "Client info not updated, skipping"
                            );
                        }
                    }
                    None => flag_or_disconnect_controller(
                        cid,
                        pending_disconnect,
                        &mut disconnected_clients,
                    ),
                }
            }
        }

        for cid in &disconnected_clients {
            state
                .unregister_client(cid, Some(DisconnectReason::NoActiveVatsimConnection))
                .await;
            updates.push(SignalingMessage::ClientDisconnected {
                id: cid.to_string(),
            });
        }

        if state.broadcast_tx.receiver_count() > 0 {
            for msg in updates {
                if let Err(err) = state.broadcast_tx.send(msg) {
                    tracing::warn!(?err, "Failed to broadcast client info update");
                }
            }
        }

        Ok(())
    }

    pub async fn health_check(&self) -> anyhow::Result<()> {
        self.store.is_healthy().await
    }

    pub fn rate_limiters(&self) -> &RateLimiters {
        &self.rate_limiters
    }
}
