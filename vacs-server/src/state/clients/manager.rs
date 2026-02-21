use crate::metrics::guards::ClientConnectionGuard;
use crate::state::clients::session::ClientSession;
use crate::state::clients::{ClientManagerError, Result};
use std::collections::{HashMap, HashSet};
use tokio::sync::broadcast::error::SendError;
use tokio::sync::{RwLock, broadcast, mpsc};
use tracing::instrument;
use vacs_protocol::profile::{ActiveProfile, ProfileId};
use vacs_protocol::vatsim::{ClientId, PositionId, StationChange, StationId};
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
    vatsim_only_positions: RwLock<HashSet<PositionId>>,
}

impl ClientManager {
    pub fn new(broadcast_tx: broadcast::Sender<ServerMessage>, network: Network) -> Self {
        Self {
            broadcast_tx,
            network,
            clients: RwLock::new(HashMap::new()),
            online_positions: RwLock::new(HashMap::new()),
            online_stations: RwLock::new(HashMap::new()),
            vatsim_only_positions: RwLock::new(HashSet::new()),
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

    /// Transforms station changes to only include changes visible to vacs clients.
    /// Stations covered solely by VATSIM-only positions are not callable, so:
    /// - `Online` for a VATSIM-only position is dropped
    /// - `Offline` for a station whose *previous* covering position was VATSIM-only is dropped
    ///   (clients never knew it was online)
    /// - `Handoff` to a VATSIM-only position becomes `Offline` (station leaves vacs coverage)
    /// - `Handoff` from a VATSIM-only position becomes `Online` (station enters vacs coverage)
    fn client_visible_changes(
        changes: &[StationChange],
        online_positions: &HashMap<PositionId, HashSet<ClientId>>,
    ) -> Vec<StationChange> {
        changes
            .iter()
            .filter_map(|change| match change {
                StationChange::Online { position_id, .. } => {
                    if online_positions.contains_key(position_id) {
                        Some(change.clone())
                    } else {
                        None
                    }
                }
                StationChange::Handoff {
                    station_id,
                    from_position_id,
                    to_position_id,
                } => {
                    let from_vacs = online_positions.contains_key(from_position_id);
                    let to_vacs = online_positions.contains_key(to_position_id);
                    match (from_vacs, to_vacs) {
                        // vacs -> vacs: normal handoff
                        (true, true) => Some(change.clone()),
                        // vacs -> VATSIM-only: station leaves vacs coverage
                        (true, false) => Some(StationChange::Offline {
                            station_id: station_id.clone(),
                        }),
                        // VATSIM-only -> vacs: station enters vacs coverage
                        (false, true) => Some(StationChange::Online {
                            station_id: station_id.clone(),
                            position_id: to_position_id.clone(),
                        }),
                        // VATSIM-only -> VATSIM-only: invisible to clients
                        (false, false) => None,
                    }
                }
                StationChange::Offline { .. } => Some(change.clone()),
            })
            .collect()
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
                let vatsim_only = self.vatsim_only_positions.read().await;
                let all_positions: HashSet<&PositionId> =
                    online_positions.keys().chain(vatsim_only.iter()).collect();
                let all_changes =
                    self.network
                        .coverage_changes(None, Some(position_id), &all_positions);
                drop(vatsim_only);

                online_positions
                    .insert(position_id.clone(), HashSet::from([client_info.id.clone()]));

                tracing::trace!(
                    ?position_id,
                    "Updating online stations list after position addition"
                );
                self.update_online_stations(&all_changes).await;

                Self::client_visible_changes(&all_changes, &online_positions)
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

                    let vatsim_only = self.vatsim_only_positions.read().await;
                    let before_all: HashSet<&PositionId> =
                        online_positions.keys().chain(vatsim_only.iter()).collect();
                    let mut after_all = before_all.clone();
                    after_all.remove(position_id);
                    let all_changes = self.network.coverage_diff(&before_all, &after_all);
                    drop(vatsim_only);

                    online_positions.remove(position_id);

                    tracing::trace!(
                        ?position_id,
                        "Updating online stations list after position removal"
                    );
                    self.update_online_stations(&all_changes).await;

                    changes.extend(Self::client_visible_changes(
                        &all_changes,
                        &online_positions,
                    ));
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

        if self.clients.read().await.is_empty() {
            tracing::debug!(
                "Last client disconnected, clearing VATSIM-only positions and online stations"
            );
            self.vatsim_only_positions.write().await.clear();
            self.online_stations.write().await.clear();
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
        let online_positions = self.online_positions.read().await;

        let mut stations: Vec<StationInfo> = match relevant_stations {
            RelevantStations::All => online_stations
                .iter()
                .filter(|(_, position_id)| online_positions.contains_key(*position_id))
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
                    online_stations.get(id).and_then(|controller| {
                        online_positions.contains_key(controller).then(|| {
                            let own = self_position_id
                                .map(|self_pos| controller == self_pos)
                                .unwrap_or(false);
                            StationInfo {
                                id: id.clone(),
                                own,
                            }
                        })
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
        require_active_connection: bool,
    ) -> Vec<(ClientId, DisconnectReason)> {
        let mut updates: Vec<ServerMessage> = Vec::new();
        let mut disconnected_clients: Vec<(ClientId, DisconnectReason)> = Vec::new();
        let mut coverage_changes: Vec<StationChange> = Vec::new();

        {
            let mut clients = self.clients.write().await;
            let mut online_positions = self.online_positions.write().await;
            let mut vatsim_only = self.vatsim_only_positions.write().await;

            let start_all_positions: HashSet<PositionId> = online_positions
                .keys()
                .chain(vatsim_only.iter())
                .cloned()
                .collect();
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
                        if require_active_connection {
                            disconnect_or_mark_pending(
                                cid,
                                pending_disconnect,
                                &mut disconnected_clients,
                            );
                        }
                    }
                    None => {
                        if require_active_connection {
                            disconnect_or_mark_pending(
                                cid,
                                pending_disconnect,
                                &mut disconnected_clients,
                            );
                        }
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

            let vacs_client_ids: HashSet<&ClientId> = clients.keys().collect();
            let mut new_vatsim_only: HashSet<PositionId> = HashSet::new();

            for (cid, controller) in controllers {
                if controller.facility_type == FacilityType::Unknown
                    || vacs_client_ids.contains(cid)
                {
                    continue;
                }
                let positions = self.network.find_positions(
                    &controller.callsign,
                    &controller.frequency,
                    controller.facility_type,
                );
                if positions.len() == 1 && !online_positions.contains_key(&positions[0].id) {
                    new_vatsim_only.insert(positions[0].id.clone());
                }
            }

            if *vatsim_only != new_vatsim_only {
                tracing::debug!(
                    before = vatsim_only.len(),
                    after = new_vatsim_only.len(),
                    "VATSIM-only positions changed"
                );
                *vatsim_only = new_vatsim_only;
                positions_changed = true;
            }

            if positions_changed {
                tracing::debug!("Online positions changed, calculating coverage changes");
                let start_all = start_all_positions.iter().collect::<HashSet<_>>();
                let end_all: HashSet<&PositionId> =
                    online_positions.keys().chain(vatsim_only.iter()).collect();

                let all_changes = self.network.coverage_diff(&start_all, &end_all);
                self.update_online_stations(&all_changes).await;

                coverage_changes.extend(Self::client_visible_changes(
                    &all_changes,
                    &online_positions,
                ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn pos(id: &str) -> PositionId {
        PositionId::from(id)
    }

    fn station(id: &str) -> StationId {
        StationId::from(id)
    }

    fn cid(id: &str) -> ClientId {
        ClientId::from(id)
    }

    fn controller(cid: &str, callsign: &str, freq: &str, ft: FacilityType) -> ControllerInfo {
        ControllerInfo {
            cid: ClientId::from(cid),
            callsign: callsign.to_string(),
            frequency: freq.to_string(),
            facility_type: ft,
        }
    }

    fn client_info(id: &str, position_id: &str, freq: &str) -> ClientInfo {
        ClientInfo {
            id: ClientId::from(id),
            position_id: Some(PositionId::from(position_id)),
            display_name: id.to_string(),
            frequency: freq.to_string(),
        }
    }

    fn online_positions(entries: &[&str]) -> HashMap<PositionId, HashSet<ClientId>> {
        entries
            .iter()
            .map(|id| (pos(id), HashSet::from([cid("1000000")])))
            .collect()
    }

    fn client_manager(network: Network) -> ClientManager {
        let (tx, _) = broadcast::channel(64);
        ClientManager::new(tx, network)
    }

    fn create_lovv_network() -> (tempfile::TempDir, Network) {
        let dir = tempfile::tempdir().unwrap();
        let fir_path = dir.path().join("LOVV");
        std::fs::create_dir(&fir_path).unwrap();

        std::fs::write(
            fir_path.join("stations.toml"),
            r#"
[[stations]]
id = "LOWW_APP"
controlled_by = ["LOWW_APP", "LOVV_CTR"]

[[stations]]
id = "LOWW_TWR"
parent_id = "LOWW_APP"
controlled_by = ["LOWW_TWR"]

[[stations]]
id = "LOWW_GND"
parent_id = "LOWW_TWR"
controlled_by = ["LOWW_GND"]

[[stations]]
id = "LOWW_DEL"
parent_id = "LOWW_GND"
controlled_by = ["LOWW_DEL"]
"#,
        )
        .unwrap();

        std::fs::write(
            fir_path.join("positions.toml"),
            r#"
[[positions]]
id = "LOVV_CTR"
prefixes = ["LOVV"]
frequency = "132.600"
facility_type = "CTR"

[[positions]]
id = "LOWW_APP"
prefixes = ["LOWW"]
frequency = "134.675"
facility_type = "APP"

[[positions]]
id = "LOWW_TWR"
prefixes = ["LOWW"]
frequency = "119.400"
facility_type = "TWR"

[[positions]]
id = "LOWW_GND"
prefixes = ["LOWW"]
frequency = "121.600"
facility_type = "GND"

[[positions]]
id = "LOWW_DEL"
prefixes = ["LOWW"]
frequency = "122.125"
facility_type = "DEL"
"#,
        )
        .unwrap();

        let network = Network::load_from_dir(dir.path()).unwrap();
        (dir, network)
    }

    #[test]
    fn online_vacs_position_is_visible() {
        let changes = vec![StationChange::Online {
            station_id: station("LOWW_TWR"),
            position_id: pos("LOWW_TWR"),
        }];
        let positions = online_positions(&["LOWW_TWR"]);

        let result = ClientManager::client_visible_changes(&changes, &positions);
        assert_eq!(result, changes);
    }

    #[test]
    fn online_vatsim_only_position_is_dropped() {
        let changes = vec![StationChange::Online {
            station_id: station("LOWW_TWR"),
            position_id: pos("LOWW_TWR"),
        }];
        let positions = online_positions(&[]);

        let result = ClientManager::client_visible_changes(&changes, &positions);
        assert!(result.is_empty());
    }

    #[test]
    fn handoff_vacs_to_vacs_is_visible() {
        let changes = vec![StationChange::Handoff {
            station_id: station("LOWW_APP"),
            from_position_id: pos("LOVV_CTR"),
            to_position_id: pos("LOWW_APP"),
        }];
        let positions = online_positions(&["LOVV_CTR", "LOWW_APP"]);

        let result = ClientManager::client_visible_changes(&changes, &positions);
        assert_eq!(result, changes);
    }

    #[test]
    fn handoff_vacs_to_vatsim_only_becomes_offline() {
        let changes = vec![StationChange::Handoff {
            station_id: station("LOWW_TWR"),
            from_position_id: pos("LOWW_APP"),
            to_position_id: pos("LOWW_TWR"),
        }];
        let positions = online_positions(&["LOWW_APP"]);

        let result = ClientManager::client_visible_changes(&changes, &positions);
        assert_eq!(
            result,
            vec![StationChange::Offline {
                station_id: station("LOWW_TWR"),
            }]
        );
    }

    #[test]
    fn handoff_vatsim_only_to_vacs_becomes_online() {
        let changes = vec![StationChange::Handoff {
            station_id: station("LOWW_TWR"),
            from_position_id: pos("LOWW_TWR"),
            to_position_id: pos("LOWW_APP"),
        }];
        let positions = online_positions(&["LOWW_APP"]);

        let result = ClientManager::client_visible_changes(&changes, &positions);
        assert_eq!(
            result,
            vec![StationChange::Online {
                station_id: station("LOWW_TWR"),
                position_id: pos("LOWW_APP"),
            }]
        );
    }

    #[test]
    fn handoff_vatsim_only_to_vatsim_only_is_dropped() {
        let changes = vec![StationChange::Handoff {
            station_id: station("LOWW_APP"),
            from_position_id: pos("LOVV_CTR"),
            to_position_id: pos("LOWW_APP"),
        }];
        let positions = online_positions(&[]);

        let result = ClientManager::client_visible_changes(&changes, &positions);
        assert!(result.is_empty());
    }

    #[test]
    fn offline_is_always_visible() {
        let changes = vec![StationChange::Offline {
            station_id: station("LOWW_TWR"),
        }];
        let positions = online_positions(&[]);

        let result = ClientManager::client_visible_changes(&changes, &positions);
        assert_eq!(result, changes);
    }

    #[tokio::test]
    async fn vatsim_only_position_removes_station_from_vacs_client() {
        let (_dir, network) = create_lovv_network();
        let manager = client_manager(network);

        let _client = manager
            .add_client(
                client_info("client0", "LOWW_APP", "134.675"),
                ActiveProfile::Custom,
                ClientConnectionGuard::default(),
            )
            .await
            .unwrap();

        // LOWW_APP should cover LOWW_APP, LOWW_TWR, LOWW_GND, LOWW_DEL stations
        let stations = manager
            .list_stations(&ActiveProfile::Custom, Some(&pos("LOWW_APP")))
            .await;
        let station_ids: Vec<&str> = stations.iter().map(|s| s.id.as_str()).collect();
        assert!(station_ids.contains(&"LOWW_APP"));
        assert!(station_ids.contains(&"LOWW_TWR"));
        assert!(station_ids.contains(&"LOWW_GND"));
        assert!(station_ids.contains(&"LOWW_DEL"));

        // Now LOWW_TWR comes online on VATSIM only (not on vacs)
        let vatsim_controllers = HashMap::from([
            (
                cid("client0"),
                controller("client0", "LOWW_APP", "134.675", FacilityType::Approach),
            ),
            (
                cid("vatsim_client1"),
                controller("vatsim_client1", "LOWW_TWR", "119.400", FacilityType::Tower),
            ),
        ]);

        let disconnected = manager
            .sync_vatsim_state(&vatsim_controllers, &mut HashSet::new(), false)
            .await;
        assert!(disconnected.is_empty());

        let stations = manager
            .list_stations(&ActiveProfile::Custom, Some(&pos("LOWW_APP")))
            .await;
        let station_ids: Vec<&str> = stations.iter().map(|s| s.id.as_str()).collect();
        assert!(station_ids.contains(&"LOWW_APP"));
        assert!(
            !station_ids.contains(&"LOWW_TWR"),
            "LOWW_TWR should not be listed (VATSIM-only)"
        );
        // LOWW_GND and LOWW_DEL are children of LOWW_TWR, now covered by VATSIM-only LOWW_TWR
        assert!(
            !station_ids.contains(&"LOWW_GND"),
            "LOWW_GND should not be listed (covered by VATSIM-only LOWW_TWR)"
        );
        assert!(
            !station_ids.contains(&"LOWW_DEL"),
            "LOWW_DEL should not be listed (covered by VATSIM-only LOWW_TWR)"
        );

        // But internally, LOWW_TWR station should be tracked in online_stations
        let internal_stations = manager.online_stations.read().await;
        assert!(internal_stations.contains_key(&station("LOWW_TWR")));
    }

    #[tokio::test]
    async fn vatsim_only_position_becomes_vacs_when_client_connects() {
        let (_dir, network) = create_lovv_network();
        let manager = client_manager(network);

        // vacs client connects as LOVV_CTR (covers everything including LOWW_APP,
        // LOWW_TWR, etc.)
        let _client = manager
            .add_client(
                client_info("client0", "LOVV_CTR", "132.600"),
                ActiveProfile::Custom,
                ClientConnectionGuard::default(),
            )
            .await
            .unwrap();

        // LOWW_TWR comes online on VATSIM only
        let vatsim_controllers = HashMap::from([
            (
                cid("client0"),
                controller("client0", "LOVV_CTR", "132.600", FacilityType::Enroute),
            ),
            (
                cid("vatsim_client1"),
                controller("vatsim_client1", "LOWW_TWR", "119.400", FacilityType::Tower),
            ),
        ]);
        manager
            .sync_vatsim_state(&vatsim_controllers, &mut HashSet::new(), false)
            .await;

        // LOWW_TWR station is NOT callable (VATSIM-only)
        let stations = manager
            .list_stations(&ActiveProfile::Custom, Some(&pos("LOVV_CTR")))
            .await;
        let station_ids: Vec<&str> = stations.iter().map(|s| s.id.as_str()).collect();
        assert!(!station_ids.contains(&"LOWW_TWR"));

        // Now a vacs client connects as LOWW_TWR
        let _client_twr = manager
            .add_client(
                client_info("client2", "LOWW_TWR", "119.400"),
                ActiveProfile::Custom,
                ClientConnectionGuard::default(),
            )
            .await
            .unwrap();

        // LOWW_TWR should now be in the list (vacs client covers it)
        let stations = manager
            .list_stations(&ActiveProfile::Custom, Some(&pos("LOVV_CTR")))
            .await;
        let station_ids: Vec<&str> = stations.iter().map(|s| s.id.as_str()).collect();
        assert!(
            station_ids.contains(&"LOWW_TWR"),
            "LOWW_TWR should be listed after vacs client connects"
        );
    }

    #[tokio::test]
    async fn vacs_client_disconnect_with_vatsim_only_covering_same_position() {
        let (_dir, network) = create_lovv_network();
        let manager = client_manager(network);

        // vacs client connects as LOVV_CTR
        let _client_ctr = manager
            .add_client(
                client_info("client0", "LOVV_CTR", "132.600"),
                ActiveProfile::Custom,
                ClientConnectionGuard::default(),
            )
            .await
            .unwrap();

        // vacs client connects as LOWW_TWR
        let _client_twr = manager
            .add_client(
                client_info("client2", "LOWW_TWR", "119.400"),
                ActiveProfile::Custom,
                ClientConnectionGuard::default(),
            )
            .await
            .unwrap();

        // LOWW_TWR is callable
        let stations = manager
            .list_stations(&ActiveProfile::Custom, Some(&pos("LOVV_CTR")))
            .await;
        let station_ids: Vec<&str> = stations.iter().map(|s| s.id.as_str()).collect();
        assert!(station_ids.contains(&"LOWW_TWR"));

        // vacs LOWW_TWR client disconnects
        manager
            .remove_client(cid("client2"), Some(DisconnectReason::Terminated))
            .await;

        // But VATSIM-only LOWW_TWR is still online
        let vatsim_controllers = HashMap::from([
            (
                cid("client0"),
                controller("client0", "LOVV_CTR", "132.600", FacilityType::Enroute),
            ),
            (
                cid("vatsim_client1"),
                controller("vatsim_client1", "LOWW_TWR", "119.400", FacilityType::Tower),
            ),
        ]);
        manager
            .sync_vatsim_state(&vatsim_controllers, &mut HashSet::new(), false)
            .await;

        // LOWW_TWR should NOT be callable (VATSIM-only now)
        let stations = manager
            .list_stations(&ActiveProfile::Custom, Some(&pos("LOVV_CTR")))
            .await;
        let station_ids: Vec<&str> = stations.iter().map(|s| s.id.as_str()).collect();
        assert!(
            !station_ids.contains(&"LOWW_TWR"),
            "LOWW_TWR should not be listed (VATSIM-only after vacs client disconnect)"
        );

        // But LOWW_TWR should still be in internal tracking
        let internal_stations = manager.online_stations.read().await;
        assert!(internal_stations.contains_key(&station("LOWW_TWR")));
    }

    #[tokio::test]
    async fn multiple_vatsim_only_positions_not_callable() {
        let (_dir, network) = create_lovv_network();
        let manager = client_manager(network);

        // vacs client connects as LOVV_CTR
        let _client = manager
            .add_client(
                client_info("client0", "LOVV_CTR", "132.600"),
                ActiveProfile::Custom,
                ClientConnectionGuard::default(),
            )
            .await
            .unwrap();

        // Both LOWW_TWR and LOWW_GND online on VATSIM only
        let vatsim_controllers = HashMap::from([
            (
                cid("client0"),
                controller("client0", "LOVV_CTR", "132.600", FacilityType::Enroute),
            ),
            (
                cid("vatsim_client1"),
                controller("vatsim_client1", "LOWW_TWR", "119.400", FacilityType::Tower),
            ),
            (
                cid("vatsim_client2"),
                controller(
                    "vatsim_client2",
                    "LOWW_GND",
                    "121.600",
                    FacilityType::Ground,
                ),
            ),
        ]);
        manager
            .sync_vatsim_state(&vatsim_controllers, &mut HashSet::new(), false)
            .await;

        let stations = manager
            .list_stations(&ActiveProfile::Custom, Some(&pos("LOVV_CTR")))
            .await;
        let station_ids: Vec<&str> = stations.iter().map(|s| s.id.as_str()).collect();

        assert!(
            !station_ids.contains(&"LOWW_TWR"),
            "LOWW_TWR should not be callable (vatsim-only)"
        );
        assert!(
            !station_ids.contains(&"LOWW_GND"),
            "LOWW_GND should not be callable (vatsim-only)"
        );
        assert!(
            !station_ids.contains(&"LOWW_DEL"),
            "LOWW_DEL should not be callable (covered by vatsim-only LOWW_GND)"
        );
        // LOWW_APP should still be covered by LOVV_CTR
        assert!(
            station_ids.contains(&"LOWW_APP"),
            "LOWW_APP should still be callable (covered by VACS LOVV_CTR)"
        );
    }

    #[tokio::test]
    async fn last_client_disconnect_clears_vatsim_only_state() {
        let (_dir, network) = create_lovv_network();
        let manager = client_manager(network);

        // vacs client connects
        let _client = manager
            .add_client(
                client_info("client0", "LOWW_APP", "134.675"),
                ActiveProfile::Custom,
                ClientConnectionGuard::default(),
            )
            .await
            .unwrap();

        // Sync with VATSIM-only TWR
        let vatsim_controllers = HashMap::from([
            (
                cid("client0"),
                controller("client0", "LOWW_APP", "134.675", FacilityType::Approach),
            ),
            (
                cid("vatsim_client1"),
                controller("vatsim_client1", "LOWW_TWR", "119.400", FacilityType::Tower),
            ),
        ]);
        manager
            .sync_vatsim_state(&vatsim_controllers, &mut HashSet::new(), false)
            .await;

        assert!(!manager.vatsim_only_positions.read().await.is_empty());
        assert!(!manager.online_stations.read().await.is_empty());

        // Last vacs client disconnects
        manager
            .remove_client(cid("client0"), Some(DisconnectReason::Terminated))
            .await;

        assert!(
            manager.vatsim_only_positions.read().await.is_empty(),
            "VATSIM-only positions should be cleared after last client disconnects"
        );
        assert!(
            manager.online_stations.read().await.is_empty(),
            "online stations should be cleared after last client disconnects"
        );
    }

    #[tokio::test]
    async fn clients_for_station_returns_empty_for_vatsim_only() {
        let (_dir, network) = create_lovv_network();
        let manager = client_manager(network);

        // vacs client connects as LOVV_CTR
        let _client = manager
            .add_client(
                client_info("client0", "LOVV_CTR", "132.600"),
                ActiveProfile::Custom,
                ClientConnectionGuard::default(),
            )
            .await
            .unwrap();

        // LOWW_TWR online VATSIM-only
        let vatsim_controllers = HashMap::from([
            (
                cid("client0"),
                controller("client0", "LOVV_CTR", "132.600", FacilityType::Enroute),
            ),
            (
                cid("vatsim_client1"),
                controller("vatsim_client1", "LOWW_TWR", "119.400", FacilityType::Tower),
            ),
        ]);
        manager
            .sync_vatsim_state(&vatsim_controllers, &mut HashSet::new(), false)
            .await;

        // LOWW_TWR station exists internally but has no callable clients
        let clients = manager.clients_for_station(&station("LOWW_TWR")).await;
        assert!(
            clients.is_empty(),
            "clients_for_station should return empty for VATSIM-only station"
        );
    }
}
