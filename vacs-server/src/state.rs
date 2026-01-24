pub mod calls;
pub mod clients;

use crate::config;
use crate::config::AppConfig;
use crate::ice::provider::IceConfigProvider;
use crate::metrics::ErrorMetrics;
use crate::metrics::guards::ClientConnectionGuard;
use crate::ratelimit::RateLimiters;
use crate::release::UpdateChecker;
use crate::state::calls::CallManager;
use crate::state::clients::{ClientManager, ClientSession};
use crate::store::{Store, StoreBackend};
use anyhow::Context;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{Instrument, instrument};
use uuid::Uuid;
use vacs_protocol::vatsim::{ActiveProfile, ClientId, PositionId, ProfileId};
use vacs_protocol::ws::server::{ClientInfo, DisconnectReason, ServerMessage, StationInfo};
use vacs_protocol::ws::shared::{Error, ErrorReason};
use vacs_vatsim::ControllerInfo;
use vacs_vatsim::coverage::network::Network;
use vacs_vatsim::data_feed::DataFeed;
use vacs_vatsim::slurper::SlurperClient;

pub struct AppState {
    pub config: AppConfig,
    pub updates: UpdateChecker,
    pub calls: CallManager,
    pub clients: ClientManager,
    pub ice_config_provider: Arc<dyn IceConfigProvider>,
    store: Store,
    broadcast_tx: broadcast::Sender<ServerMessage>,
    slurper: SlurperClient,
    data_feed: Arc<dyn DataFeed>,
    rate_limiters: RateLimiters,
    shutdown_rx: watch::Receiver<()>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: AppConfig,
        updates: UpdateChecker,
        store: Store,
        slurper: SlurperClient,
        data_feed: Arc<dyn DataFeed>,
        network: Network,
        rate_limiters: RateLimiters,
        shutdown_rx: watch::Receiver<()>,
        ice_config_provider: Arc<dyn IceConfigProvider>,
    ) -> Self {
        let (broadcast_tx, _) = broadcast::channel(config::BROADCAST_CHANNEL_CAPACITY);
        Self {
            config,
            updates,
            ice_config_provider,
            store,
            calls: CallManager::new(),
            clients: ClientManager::new(broadcast_tx.clone(), network),
            broadcast_tx,
            slurper,
            data_feed,
            rate_limiters,
            shutdown_rx,
        }
    }

    pub fn get_client_receivers(
        &self,
    ) -> (broadcast::Receiver<ServerMessage>, watch::Receiver<()>) {
        (self.broadcast_tx.subscribe(), self.shutdown_rx.clone())
    }

    #[instrument(level = "debug", skip(self, client_connection_guard), err)]
    pub async fn register_client(
        &self,
        client_info: ClientInfo,
        active_profile: ActiveProfile<ProfileId>,
        client_connection_guard: ClientConnectionGuard,
    ) -> anyhow::Result<(ClientSession, mpsc::Receiver<ServerMessage>)> {
        tracing::trace!("Registering client");

        let (client, rx) = self
            .clients
            .add_client(client_info, active_profile, client_connection_guard)
            .await?;

        tracing::trace!("Client registered");
        Ok((client, rx))
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn unregister_client(
        &self,
        client_id: &ClientId,
        disconnect_reason: Option<DisconnectReason>,
    ) {
        tracing::trace!("Unregistering client");

        self.clients
            .remove_client(client_id.clone(), disconnect_reason)
            .await;

        self.calls.cleanup_client_calls(client_id);

        tracing::debug!("Client unregistered");
    }

    pub async fn list_clients(&self, self_client_id: Option<&ClientId>) -> Vec<ClientInfo> {
        self.clients.list_clients(self_client_id).await
    }

    pub async fn list_stations(
        &self,
        active_profile: &ActiveProfile<ProfileId>,
        self_position_id: Option<&PositionId>,
    ) -> Vec<StationInfo> {
        self.clients
            .list_stations(active_profile, self_position_id)
            .await
    }

    pub async fn get_client(&self, client_id: &ClientId) -> Option<ClientSession> {
        self.clients.get_client(client_id).await
    }

    #[tracing::instrument(level = "trace", skip(self, message))]
    pub async fn send_message(
        &self,
        client_id: &ClientId,
        message: impl Into<ServerMessage>,
    ) -> Result<(), Error> {
        match self.get_client(client_id).await {
            Some(client) => {
                tracing::trace!("Sending message to client");
                if let Err(err) = client.send_message(message).await {
                    tracing::warn!(?err, "Failed to send message to client");
                    ErrorMetrics::error(&ErrorReason::PeerConnection);
                    Err(Error::new(ErrorReason::PeerConnection).with_client_id(client_id.clone()))
                } else {
                    Ok(())
                }
            }
            None => {
                tracing::warn!("Client not found");
                ErrorMetrics::peer_not_found();
                Err(Error::new(ErrorReason::ClientNotFound).with_client_id(client_id.clone()))
            }
        }
    }

    pub async fn send_peer_message(
        &self,
        source: &ClientSession,
        target: &ClientId,
        message: impl Into<ServerMessage>,
    ) {
        if let Err(err) = self.send_message(target, message).await
            && source.send_message(err).await.is_err()
        {
            tracing::warn!("Failed to send error message to source client");
        }
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn generate_ws_auth_token(&self, cid: &str) -> anyhow::Result<String> {
        tracing::debug!("Generating web socket auth token");

        let token = Uuid::now_v7().to_string();

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
    pub async fn verify_ws_auth_token(&self, token: &str) -> anyhow::Result<ClientId> {
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
        cid: &ClientId,
    ) -> anyhow::Result<Option<ControllerInfo>> {
        tracing::debug!("Retrieving connection info from VATSIM slurper");
        self.slurper
            .get_controller_info(cid)
            .await
            .map_err(Into::into)
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn get_vatsim_controllers(&self) -> anyhow::Result<Vec<ControllerInfo>> {
        tracing::debug!("Retrieving controller info from VATSIM data feed");
        self.data_feed
            .fetch_controller_info()
            .await
            .map_err(Into::into)
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
                            if state.clients.is_empty().await {
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

    pub async fn force_update_controllers(state: &Arc<AppState>) -> anyhow::Result<()> {
        let mut pending_disconnect = HashSet::new();
        Self::update_vatsim_controllers(state, &mut pending_disconnect).await
    }

    async fn update_vatsim_controllers(
        state: &Arc<AppState>,
        pending_disconnect: &mut HashSet<ClientId>,
    ) -> anyhow::Result<()> {
        let controllers = state.get_vatsim_controllers().await?;
        let current: HashMap<ClientId, ControllerInfo> = controllers
            .into_iter()
            .map(|c| (c.cid.clone(), c))
            .collect();

        let disconnected_clients = state
            .clients
            .sync_vatsim_state(&current, pending_disconnect)
            .await;

        for (cid, disconnect_reason) in disconnected_clients {
            state.unregister_client(&cid, Some(disconnect_reason)).await;
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
