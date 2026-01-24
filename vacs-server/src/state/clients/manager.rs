use crate::metrics::guards::ClientConnectionGuard;
use crate::state::clients::session::ClientSession;
use crate::state::clients::{ClientManagerError, Result};
use std::collections::{HashMap, HashSet};
use tokio::sync::broadcast::error::SendError;
use tokio::sync::{RwLock, broadcast, mpsc};
use tracing::instrument;
use vacs_protocol::vatsim::{
    ActiveProfile, ClientId, PositionId, ProfileId, StationChange, StationId,
};
use vacs_protocol::ws::server;
use vacs_protocol::ws::server::{ClientInfo, DisconnectReason, ServerMessage, StationInfo};
use vacs_vatsim::coverage::network::{Network, RelevantStations};
use vacs_vatsim::coverage::position::Position;
use vacs_vatsim::coverage::profile::Profile;
use vacs_vatsim::{ControllerInfo, FacilityType};

#[derive(Debug)]
pub struct ClientManager {
    broadcast_tx: broadcast::Sender<ServerMessage>,
    network: Network,
    clients: RwLock<HashMap<ClientId, ClientSession>>,
    online_positions: RwLock<HashMap<PositionId, HashSet<ClientId>>>,
    online_stations: RwLock<HashMap<StationId, PositionId>>,
}

impl ClientManager {
    pub fn new(broadcast_tx: broadcast::Sender<ServerMessage>, network: Network) -> Self {
        Self {
            broadcast_tx,
            network,
            clients: RwLock::new(HashMap::new()),
            online_positions: RwLock::new(HashMap::new()),
            online_stations: RwLock::new(HashMap::new()),
        }
    }

    #[instrument(level = "debug", skip(self))]
    pub fn find_positions(&self, controller_info: &ControllerInfo) -> Vec<&Position> {
        self.network.find_positions(
            &controller_info.callsign,
            &controller_info.frequency,
            controller_info.facility_type,
        )
    }

    pub fn get_profile(&self, profile_id: Option<&ProfileId>) -> Option<&Profile> {
        profile_id.and_then(|profile_id| self.network.get_profile(profile_id))
    }

    pub fn get_position(&self, position_id: Option<&PositionId>) -> Option<&Position> {
        position_id.and_then(|position_id| self.network.get_position(position_id))
    }

    pub async fn clients_for_position(&self, position_id: &PositionId) -> HashSet<ClientId> {
        self.online_positions
            .read()
            .await
            .get(position_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn clients_for_station(&self, station_id: &StationId) -> HashSet<ClientId> {
        let Some(position_id) = self.online_stations.read().await.get(station_id).cloned() else {
            return HashSet::new();
        };
        self.clients_for_position(&position_id).await
    }

    pub fn network(&self) -> &Network {
        &self.network
    }

    #[instrument(level = "debug", skip(self, client_connection_guard), err)]
    pub async fn add_client(
        &self,
        client_info: ClientInfo,
        active_profile: ActiveProfile<ProfileId>,
        client_connection_guard: ClientConnectionGuard,
    ) -> Result<(ClientSession, mpsc::Receiver<ServerMessage>)> {
        tracing::trace!("Adding client");

        if self.clients.read().await.contains_key(&client_info.id) {
            tracing::trace!("Client already exists");
            return Err(ClientManagerError::DuplicateClient(
                client_info.id.to_string(),
            ));
        }

        let (tx, rx) = mpsc::channel(crate::config::CLIENT_CHANNEL_CAPACITY);

        let client = ClientSession::new(
            client_info.clone(),
            active_profile,
            tx,
            client_connection_guard,
        );
        self.clients
            .write()
            .await
            .insert(client_info.id.clone(), client.clone());

        let changes = if let Some(position_id) = client.position_id() {
            let mut online_positions = self.online_positions.write().await;

            let exists_and_not_empty = online_positions
                .get(position_id)
                .map(|c| !c.is_empty())
                .unwrap_or(false);

            if exists_and_not_empty {
                tracing::trace!(
                    ?position_id,
                    "Position already exists in online positions list, adding client to list of controllers"
                );
                online_positions
                    .get_mut(position_id)
                    .unwrap()
                    .insert(client_info.id.clone());
                Vec::new()
            } else {
                tracing::trace!(?position_id, "Adding position to online positions list");
                let current_positions = online_positions.keys().collect::<HashSet<_>>();
                let changes =
                    self.network
                        .coverage_changes(None, Some(position_id), &current_positions);
                online_positions
                    .insert(position_id.clone(), HashSet::from([client_info.id.clone()]));

                tracing::trace!(
                    ?position_id,
                    "Updating online stations list after position addition"
                );
                self.update_online_stations(&changes).await;

                changes
            }
        } else {
            tracing::trace!(
                "Client has no position, skipping online positions list addition and station changes broadcast"
            );
            Vec::new()
        };

        if let Err(err) = self.broadcast(server::ClientConnected {
            client: client_info,
        }) {
            tracing::warn!(?err, "Failed to broadcast client connected message");
        }

        self.broadcast_station_changes(&changes).await;

        tracing::trace!("Client added");
        Ok((client, rx))
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn remove_client(
        &self,
        client_id: ClientId,
        disconnect_reason: Option<DisconnectReason>,
    ) {
        tracing::trace!("Removing client");

        let Some(client) = self.clients.write().await.remove(&client_id) else {
            tracing::debug!("Client not found in client list, skipping removal");
            return;
        };

        let changes = if let Some(position_id) = client.position_id() {
            let mut online_positions = self.online_positions.write().await;

            if online_positions.contains_key(position_id) {
                let mut changes = Vec::new();

                if online_positions.get(position_id).unwrap().len() == 1 {
                    tracing::trace!(?position_id, "Removing position from online positions list");

                    let current_positions = online_positions.keys().collect::<HashSet<_>>();
                    let changes_vec =
                        self.network
                            .coverage_changes(Some(position_id), None, &current_positions);

                    online_positions.remove(position_id);

                    tracing::trace!(
                        ?position_id,
                        "Updating online stations list after position removal"
                    );
                    self.update_online_stations(&changes_vec).await;

                    changes.extend(changes_vec);
                } else {
                    tracing::trace!(
                        ?position_id,
                        "Removing client from position in online positions list"
                    );
                    online_positions
                        .get_mut(position_id)
                        .unwrap()
                        .remove(&client_id);
                }

                changes
            } else {
                tracing::trace!(
                    ?position_id,
                    "Position not found in online positions list, skipping removal of client from list of controllers"
                );
                Vec::new()
            }
        } else {
            tracing::trace!(
                "Client has no position, skipping online positions list removal and station changes broadcast"
            );
            Vec::new()
        };
        client.disconnect(disconnect_reason);

        if let Err(err) = self.broadcast(server::ClientDisconnected { client_id }) {
            tracing::warn!(?err, "Failed to broadcast client disconnected message");
        }

        self.broadcast_station_changes(&changes).await;

        tracing::debug!("Client removed");
    }

    pub async fn list_clients(&self, self_client_id: Option<&ClientId>) -> Vec<ClientInfo> {
        let mut clients: Vec<ClientInfo> = self
            .clients
            .read()
            .await
            .values()
            .filter(|c| self_client_id.map(|s| s != c.id()).unwrap_or(true))
            .map(|c| c.client_info().clone())
            .collect();

        clients.sort_by(|a, b| a.id.cmp(&b.id));
        clients
    }

    pub async fn list_stations(
        &self,
        profile: &ActiveProfile<ProfileId>,
        self_position_id: Option<&PositionId>,
    ) -> Vec<StationInfo> {
        let relevant_stations = self.network.relevant_stations(profile);
        let online_stations = self.online_stations.read().await;

        let mut stations: Vec<StationInfo> = match relevant_stations {
            RelevantStations::All => online_stations
                .iter()
                .map(|(id, controller)| {
                    let own = self_position_id
                        .map(|self_pos| controller == self_pos)
                        .unwrap_or(false);
                    StationInfo {
                        id: id.clone(),
                        own,
                    }
                })
                .collect(),
            RelevantStations::Subset(ids) => ids
                .iter()
                .filter_map(|id| {
                    online_stations.get(id).map(|controller| {
                        let own = self_position_id
                            .map(|self_pos| controller == self_pos)
                            .unwrap_or(false);
                        StationInfo {
                            id: id.clone(),
                            own,
                        }
                    })
                })
                .collect(),
            RelevantStations::None => Vec::new(),
        };

        stations.sort_by(|a, b| a.id.cmp(&b.id));
        stations
    }

    pub async fn get_client(&self, client_id: &ClientId) -> Option<ClientSession> {
        self.clients.read().await.get(client_id).cloned()
    }

    pub async fn is_client_connected(&self, client_id: &ClientId) -> bool {
        self.clients.read().await.contains_key(client_id)
    }

    pub async fn is_empty(&self) -> bool {
        self.clients.read().await.is_empty()
    }

    #[allow(clippy::result_large_err)]
    pub fn broadcast(
        &self,
        message: impl Into<ServerMessage>,
    ) -> Result<usize, SendError<ServerMessage>> {
        let message = message.into();
        if self.broadcast_tx.receiver_count() > 0 {
            tracing::trace!(message_variant = message.variant(), "Broadcasting message");
            self.broadcast_tx.send(message)
        } else {
            tracing::trace!(
                message_variant = message.variant(),
                "No receivers subscribed, skipping message broadcast"
            );
            Ok(0)
        }
    }

    pub async fn sync_vatsim_state(
        &self,
        controllers: &HashMap<ClientId, ControllerInfo>,
        pending_disconnect: &mut HashSet<ClientId>,
    ) -> Vec<(ClientId, DisconnectReason)> {
        let mut updates: Vec<ServerMessage> = Vec::new();
        let mut disconnected_clients: Vec<(ClientId, DisconnectReason)> = Vec::new();
        let mut coverage_changes: Vec<StationChange> = Vec::new();

        {
            let mut clients = self.clients.write().await;
            let mut online_positions = self.online_positions.write().await;
            let start_online_positions = online_positions.keys().cloned().collect::<HashSet<_>>();
            let mut positions_changed = false;

            fn disconnect_or_mark_pending(
                cid: &ClientId,
                pending_disconnect: &mut HashSet<ClientId>,
                disconnected_clients: &mut Vec<(ClientId, DisconnectReason)>,
            ) {
                if pending_disconnect.remove(cid) {
                    tracing::trace!(
                        ?cid,
                        "No active VATSIM connection found after grace period, disconnecting client and sending broadcast"
                    );
                    disconnected_clients
                        .push((cid.clone(), DisconnectReason::NoActiveVatsimConnection));
                } else {
                    tracing::trace!(
                        ?cid,
                        "Client not found in data feed, but active VATSIM connection is required, marking for disconnect"
                    );
                    pending_disconnect.insert(cid.clone());
                }
            }

            for (cid, session) in clients.iter_mut() {
                tracing::trace!(?cid, ?session, "Checking session for client info update");

                match controllers.get(cid) {
                    Some(controller) if controller.facility_type == FacilityType::Unknown => {
                        disconnect_or_mark_pending(
                            cid,
                            pending_disconnect,
                            &mut disconnected_clients,
                        );
                    }
                    None => {
                        disconnect_or_mark_pending(
                            cid,
                            pending_disconnect,
                            &mut disconnected_clients,
                        );
                    }
                    Some(controller) => {
                        if pending_disconnect.remove(cid) {
                            tracing::trace!(
                                ?cid,
                                "Found active VATSIM connection for client again, removing pending disconnect"
                            );
                        }

                        let updated = session.update_client_info(controller);
                        if updated {
                            tracing::trace!(
                                ?cid,
                                ?session,
                                "Client info updated, updating position"
                            );

                            let old_position_id = session.position_id().cloned();
                            let new_positions = self.network.find_positions(
                                &controller.callsign,
                                &controller.frequency,
                                controller.facility_type,
                            );

                            let new_position = if new_positions.len() > 1 {
                                tracing::info!(
                                    ?cid,
                                    ?old_position_id,
                                    ?new_positions,
                                    "Multiple positions found for updated client info, disconnecting as ambiguous"
                                );
                                pending_disconnect.remove(cid);
                                disconnected_clients.push((
                                    cid.clone(),
                                    DisconnectReason::AmbiguousVatsimPosition(
                                        new_positions.into_iter().map(|p| p.id.clone()).collect(),
                                    ),
                                ));
                                continue;
                            } else if new_positions.len() == 1 {
                                Some(new_positions[0])
                            } else {
                                None
                            };
                            let new_position_id = new_position.map(|p| p.id.clone());

                            if old_position_id != new_position_id {
                                tracing::info!(
                                    ?cid,
                                    ?new_position_id,
                                    ?old_position_id,
                                    "Client position changed"
                                );

                                session.set_position_id(new_position_id.clone());

                                if let Some(old_position_id) = &old_position_id {
                                    if online_positions
                                        .get(old_position_id)
                                        .map(|s| s.len() <= 1)
                                        .unwrap_or(false)
                                    {
                                        tracing::trace!(
                                            ?cid,
                                            ?old_position_id,
                                            "Removing position from online positions list"
                                        );
                                        online_positions.remove(old_position_id);
                                        positions_changed = true;
                                    } else if let Some(clients) =
                                        online_positions.get_mut(old_position_id)
                                    {
                                        tracing::trace!(
                                            ?cid,
                                            ?old_position_id,
                                            "Removing client from position in online positions list"
                                        );
                                        clients.remove(cid);
                                    }
                                }

                                if let Some(new_position_id) = &new_position_id {
                                    let clients = online_positions
                                        .entry(new_position_id.clone())
                                        .or_default();
                                    if clients.insert(cid.clone()) && clients.len() == 1 {
                                        positions_changed = true;
                                    }
                                }

                                let session_profile = session.update_active_profile(
                                    new_position.and_then(|p| p.profile_id.clone()),
                                    &self.network,
                                );

                                if let Err(err) = session
                                    .send_message(server::SessionInfo {
                                        client: session.client_info().clone(),
                                        profile: session_profile,
                                    })
                                    .await
                                {
                                    tracing::warn!(
                                        ?err,
                                        ?session,
                                        "Failed to send updated session info to client"
                                    );
                                }
                            }

                            tracing::trace!(?cid, ?session, "Client info updated, broadcasting");
                            updates.push(ServerMessage::from(session.client_info().clone()));
                        }
                    }
                }
            }

            if positions_changed {
                tracing::debug!("Online positions changed, calculating coverage changes");
                let start_online_positions = start_online_positions.iter().collect::<HashSet<_>>();
                let end_online_positions = online_positions.keys().collect::<HashSet<_>>();

                let changes = self
                    .network
                    .coverage_diff(&start_online_positions, &end_online_positions);
                self.update_online_stations(&changes).await;
                coverage_changes.extend(changes);
            }
        }

        if self.broadcast_tx.receiver_count() > 0 {
            for msg in updates {
                if let Err(err) = self.broadcast(msg) {
                    tracing::warn!(?err, "Failed to broadcast client info update");
                }
            }
        }

        self.broadcast_station_changes(&coverage_changes).await;

        disconnected_clients
    }

    async fn update_online_stations(&self, changes: &[StationChange]) {
        if changes.is_empty() {
            return;
        }

        let mut online_stations = self.online_stations.write().await;
        for change in changes {
            match change {
                StationChange::Online {
                    station_id,
                    position_id,
                } => {
                    online_stations.insert(station_id.clone(), position_id.clone());
                }
                StationChange::Offline { station_id } => {
                    online_stations.remove(station_id);
                }
                StationChange::Handoff {
                    station_id,
                    to_position_id,
                    ..
                } => {
                    online_stations.insert(station_id.clone(), to_position_id.clone());
                }
            }
        }
    }

    async fn broadcast_station_changes(&self, changes: &[StationChange]) {
        if changes.is_empty() {
            return;
        }

        tracing::trace!("Sending station changes to clients");
        let mut filtered_changes_cache: HashMap<ActiveProfile<ProfileId>, Vec<StationChange>> =
            HashMap::new();

        let clients = self
            .clients
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();

        for client in clients {
            let profile = client.active_profile();

            let changes_to_send = if let Some(cached_changes) = filtered_changes_cache.get(profile)
            {
                cached_changes.clone()
            } else {
                let relevant_stations = self.network.relevant_stations(profile);

                let filtered_changes = match relevant_stations {
                    RelevantStations::All => changes.to_vec(),
                    RelevantStations::Subset(relevant_ids) => changes
                        .iter()
                        .filter(|change| {
                            let station_id = match change {
                                StationChange::Online { station_id, .. } => station_id,
                                StationChange::Offline { station_id } => station_id,
                                StationChange::Handoff { station_id, .. } => station_id,
                            };
                            relevant_ids.contains(station_id)
                        })
                        .cloned()
                        .collect(),
                    RelevantStations::None => Vec::new(),
                };

                filtered_changes_cache.insert(profile.clone(), filtered_changes.clone());
                filtered_changes
            };

            if changes_to_send.is_empty() {
                continue;
            }

            if let Err(err) = client
                .send_message(server::StationChanges {
                    changes: changes_to_send,
                })
                .await
            {
                tracing::warn!(?err, ?client, "Failed to send station changes to client");
            }
        }
    }
}
