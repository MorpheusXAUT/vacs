use crate::metrics::guards::ClientConnectionGuard;
use crate::state::client::session::ClientSession;
use crate::state::client::{ClientManagerError, Result};
use std::collections::{HashMap, HashSet};
use tokio::sync::{RwLock, broadcast, mpsc};
use tracing::instrument;
use vacs_protocol::vatsim::{ClientId, PositionId, StationChange};
use vacs_protocol::ws::{ClientInfo, DisconnectReason, SignalingMessage};
use vacs_vatsim::coverage::network::{Network, ProfileSelection, RelevantStations};
use vacs_vatsim::{ControllerInfo, FacilityType};

#[derive(Debug)]
pub struct ClientManager {
    broadcast_tx: broadcast::Sender<SignalingMessage>,
    network: Network,
    clients: RwLock<HashMap<ClientId, ClientSession>>,
    online_positions: RwLock<HashMap<PositionId, HashSet<ClientId>>>,
}

impl ClientManager {
    pub fn new(broadcast_tx: broadcast::Sender<SignalingMessage>, network: Network) -> Self {
        Self {
            broadcast_tx,
            network,
            clients: RwLock::new(HashMap::new()),
            online_positions: RwLock::new(HashMap::new()),
        }
    }

    #[instrument(level = "debug", skip(self, client_connection_guard), err)]
    pub async fn add_client(
        &self,
        client_info: ClientInfo,
        profile_selection: ProfileSelection,
        client_connection_guard: ClientConnectionGuard,
    ) -> Result<(ClientSession, mpsc::Receiver<SignalingMessage>)> {
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
            profile_selection,
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
                changes
            }
        } else {
            tracing::trace!(
                "Client has no position, skipping online positions list addition and station changes broadcast"
            );
            Vec::new()
        };

        if self.broadcast_tx.receiver_count() > 0 {
            tracing::trace!("Broadcasting client connected message");
            if let Err(err) = self.broadcast_tx.send(SignalingMessage::ClientConnected {
                client: client_info,
            }) {
                tracing::warn!(?err, "Failed to broadcast client connected message");
            }
        } else {
            tracing::trace!(
                "No other broadcast receivers subscribed, skipping client connected message"
            );
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
                let changes = {
                    let current_positions = online_positions.keys().collect::<HashSet<_>>();
                    self.network
                        .coverage_changes(Some(position_id), None, &current_positions)
                };

                if online_positions.get(position_id).unwrap().len() == 1 {
                    tracing::trace!(?position_id, "Removing position from online positions list");
                    online_positions.remove(position_id);
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

        if self.broadcast_tx.receiver_count() > 1 {
            tracing::trace!("Broadcasting client disconnected message");
            if let Err(err) = self
                .broadcast_tx
                .send(SignalingMessage::ClientDisconnected { id: client_id })
            {
                tracing::warn!(?err, "Failed to broadcast client disconnected message");
            }
        } else {
            tracing::debug!(
                "No other broadcast receivers subscribed, skipping client disconnected message"
            );
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

    pub async fn get_client(&self, client_id: &ClientId) -> Option<ClientSession> {
        self.clients.read().await.get(client_id).cloned()
    }

    pub async fn is_empty(&self) -> bool {
        self.clients.read().await.is_empty()
    }

    pub async fn sync_vatsim_state(
        &self,
        controllers: &HashMap<ClientId, ControllerInfo>,
        pending_disconnect: &mut HashSet<ClientId>,
    ) -> Vec<ClientId> {
        let mut updates: Vec<SignalingMessage> = Vec::new();
        let mut disconnected_clients: Vec<ClientId> = Vec::new();
        let mut coverage_changes: Vec<StationChange> = Vec::new();

        {
            let mut clients = self.clients.write().await;
            let mut online_positions = self.online_positions.write().await;
            let start_online_positions = online_positions.keys().cloned().collect::<HashSet<_>>();
            let mut positions_changed = false;

            for (cid, session) in clients.iter_mut() {
                tracing::trace!(?cid, ?session, "Checking session for client info update");

                match controllers.get(cid) {
                    Some(controller) if controller.facility_type == FacilityType::Unknown => {
                        if pending_disconnect.remove(cid) {
                            tracing::trace!(
                                ?cid,
                                "No active VATSIM connection found after grace period, disconnecting client and sending broadcast"
                            );
                            disconnected_clients.push(cid.clone());
                        } else {
                            tracing::trace!(
                                ?cid,
                                "Client not found in data feed, but active VATSIM connection is required, marking for disconnect"
                            );
                            pending_disconnect.insert(cid.clone());
                        }
                    }
                    Some(controller) => {
                        if pending_disconnect.remove(cid) {
                            tracing::trace!(
                                ?cid,
                                "Found active VATSIM connection for client again, removing pending disconnect"
                            );
                        }

                        let mut changed = session.update_client_info(controller);

                        let old_position_id = session.position_id().cloned();
                        let found_positions = self.network.find_positions(
                            &controller.callsign,
                            &controller.frequency,
                            controller.facility_type,
                        );

                        if found_positions.len() > 1 && old_position_id.is_some() {
                            tracing::info!(
                                ?cid,
                                ?old_position_id,
                                matches = ?found_positions.len(),
                                "Ambiguous position match for existing client, disconnecting"
                            );
                            disconnected_clients.push(cid.clone());
                            continue;
                        }

                        let new_position_id = if found_positions.len() == 1 {
                            Some(found_positions[0].id.clone())
                        } else {
                            None
                        };

                        if old_position_id != new_position_id {
                            tracing::info!(
                                ?cid,
                                ?new_position_id,
                                ?old_position_id,
                                "Client position changed"
                            );

                            if let Some(old_position_id) = &old_position_id {
                                if online_positions
                                    .get(old_position_id)
                                    .map(|s| s.len() <= 1)
                                    .unwrap_or(false)
                                {
                                    online_positions.remove(old_position_id);
                                    positions_changed = true;
                                } else if let Some(clients) =
                                    online_positions.get_mut(old_position_id)
                                {
                                    clients.remove(cid);
                                }
                            }

                            if let Some(new_position_id) = &new_position_id {
                                let clients =
                                    online_positions.entry(new_position_id.clone()).or_default();
                                if clients.insert(cid.clone()) && clients.len() == 1 {
                                    positions_changed = true;
                                }
                            }

                            session.set_position_id(new_position_id);
                            changed = true;
                        }

                        if changed {
                            tracing::trace!(?cid, ?session, "Client info updated, broadcasting");
                            updates.push(SignalingMessage::ClientInfo {
                                own: false,
                                info: session.client_info().clone(),
                            });
                        }
                    }
                    None => {
                        if pending_disconnect.remove(cid) {
                            tracing::trace!(
                                ?cid,
                                "No active VATSIM connection found, disconnecting client and sending broadcast"
                            );
                            disconnected_clients.push(cid.clone());
                        } else {
                            tracing::trace!(
                                ?cid,
                                "Client not found in data feed, but active VATSIM connection is required, marking for disconnect"
                            );
                            pending_disconnect.insert(cid.clone());
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
                coverage_changes.extend(changes);
            }
        }

        if self.broadcast_tx.receiver_count() > 0 {
            for msg in updates {
                if let Err(err) = self.broadcast_tx.send(msg) {
                    tracing::warn!(?err, "Failed to broadcast client info update");
                }
            }
        }

        self.broadcast_station_changes(&coverage_changes).await;

        disconnected_clients
    }

    async fn broadcast_station_changes(&self, changes: &[StationChange]) {
        if changes.is_empty() {
            return;
        }

        tracing::trace!("Sending station changes to clients");
        let mut filtered_changes_cache: HashMap<ProfileSelection, Vec<StationChange>> =
            HashMap::new();

        let clients = self
            .clients
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();

        for client in clients {
            let profile_selection = client.profile_selection();

            let changes_to_send = if let Some(cached_changes) =
                filtered_changes_cache.get(profile_selection)
            {
                cached_changes.clone()
            } else {
                let relevant_stations = self.network.relevant_stations(profile_selection);

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

                filtered_changes_cache.insert(profile_selection.clone(), filtered_changes.clone());
                filtered_changes
            };

            if changes_to_send.is_empty() {
                continue;
            }

            if let Err(err) = client
                .send_message(SignalingMessage::StationChanges {
                    changes: changes_to_send,
                })
                .await
            {
                tracing::warn!(?err, ?client, "Failed to send station changes to client");
            }
        }
    }
}
