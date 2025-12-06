use crate::radio::{Radio, RadioError, RadioState, TransmissionState};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use trackaudio::messages::events::StationState;
use trackaudio::{ClientEvent, ConnectionState, TrackAudioClient, TrackAudioError};

#[derive(Clone)]
pub struct TrackAudioRadio {
    #[allow(dead_code)]
    app: AppHandle,
    client: TrackAudioClient,
    active: Arc<AtomicBool>,
    state: Arc<TrackAudioState>,
    cancellation_token: CancellationToken,
}

impl TrackAudioRadio {
    const TRANSMIT_TIMEOUT: Duration = Duration::from_millis(250);
    const VOICE_CONNECTED_STATE_TIMEOUT: Duration = Duration::from_millis(250);
    const STATION_STATES_TIMEOUT: Duration = Duration::from_millis(250);

    pub async fn new(
        app: AppHandle,
        endpoint: Option<impl AsRef<str>>,
    ) -> Result<Self, RadioError> {
        app.emit("radio:state", RadioState::Disconnected).ok();

        let client = match endpoint {
            Some(endpoint) => TrackAudioClient::connect_url(endpoint).await,
            None => TrackAudioClient::connect_default().await,
        }
        .map_err(|err| {
            app.emit("radio:state", RadioState::Error).ok();
            RadioError::Integration(format!("Failed to connect to TrackAudio: {err}"))
        })?;

        let cancellation_token = CancellationToken::new();

        let active = Arc::new(AtomicBool::new(false));
        let state = Arc::new(TrackAudioState::default());

        {
            let app = app.clone();
            let client = client.clone();
            let token = cancellation_token.clone();
            let state = state.clone();

            tauri::async_runtime::spawn(async move {
                Self::events_task(app, client, token, state).await;
            });
        }

        let radio = Self {
            app,
            client,
            active,
            state,
            cancellation_token,
        };

        Ok(radio)
    }

    async fn events_task(
        app: AppHandle,
        client: TrackAudioClient,
        cancellation_token: CancellationToken,
        state: Arc<TrackAudioState>,
    ) {
        log::debug!("Starting TrackAudio events task");

        let mut events = client.subscribe();
        loop {
            tokio::select! {
                biased;
                _ = cancellation_token.cancelled() => {
                    log::info!("TrackAudio events task cancelled");
                    break;
                }
                result = events.recv() => {
                    match result {
                        Ok(event) => Self::handle_event(event, &state, &app, &client).await,
                        Err(err) => {
                            log::error!("Error receiving TrackAudio event: {err}");
                            state.clear();
                            app.emit("radio:state", RadioState::Error).ok();
                            break;
                        }
                    }
                }
            }
        }

        log::debug!("TrackAudio events task ended");
    }

    async fn handle_event(
        event: trackaudio::Event,
        state: &TrackAudioState,
        app: &AppHandle,
        client: &TrackAudioClient,
    ) {
        use trackaudio::Event;

        match event {
            Event::TxBegin(_) => {
                log::trace!("TrackAudio transmission started");
                state.set_transmitting(true, app);
            }
            Event::TxEnd(_) => {
                log::trace!("TrackAudio transmission ended");
                state.set_transmitting(false, app);
            }
            Event::RxBegin(_) => {
                log::trace!("TrackAudio reception started");
                state.set_receiving(true, app);
            }
            Event::RxEnd(_) => {
                log::trace!("TrackAudio reception ended");
                state.set_receiving(false, app);
            }
            Event::VoiceConnectedState(payload) => {
                log::trace!("TrackAudio voice connection state changed: {payload:?}");
                state.set_voice_connected(payload.connected, app);
            }
            Event::Client(ClientEvent::ConnectionStateChanged(connection_state)) => {
                Self::handle_connection_state(connection_state, state, app, client).await;
            }
            Event::Client(ClientEvent::CommandSendFailed { .. })
            | Event::Client(ClientEvent::EventDeserializationFailed { .. }) => {
                log::warn!("TrackAudio client error event");
                state.clear();
                app.emit("radio:state", RadioState::Error).ok();
            }
            Event::StationAdded(payload) => {
                log::trace!("TrackAudio station added: {}", payload.callsign);
                state.add_station(payload.callsign, app);
            }
            Event::StationStateUpdate(payload) => {
                log::trace!("TrackAudio station state update: {payload:?}");
                if let Some(callsign) = payload.callsign {
                    state.update_station(callsign, payload.rx, payload.is_available, app);
                }
            }
            Event::StationStates(payload) => {
                log::trace!(
                    "Received full station states list for {} stations",
                    payload.stations.len()
                );
                state.sync_stations(payload.stations.into_iter().map(|s| s.value).collect(), app);
            }
            _ => {
                log::trace!("Received TrackAudio event: {event:?}");
            }
        }
    }

    async fn handle_connection_state(
        connection_state: ConnectionState,
        state: &TrackAudioState,
        app: &AppHandle,
        client: &TrackAudioClient,
    ) {
        match connection_state {
            ConnectionState::Connected => {
                log::trace!("Successfully connected to TrackAudio");
                state.set_connected(true, app); // This will emit, but we do more specific emissions after fetch

                let api = client.api();
                let voice_connected = api
                    .get_voice_connected_state(Some(Self::VOICE_CONNECTED_STATE_TIMEOUT))
                    .await
                    .unwrap_or(false);
                state
                    .voice_connected
                    .store(voice_connected, Ordering::Relaxed);

                let station_states = api
                    .get_station_states(Some(Self::STATION_STATES_TIMEOUT))
                    .await
                    .unwrap_or_default();

                state.sync_stations(station_states, app);
            }
            ConnectionState::Connecting { .. } | ConnectionState::Reconnecting { .. } => {
                log::trace!("Connecting to TrackAudio");
                state.clear();
                state.emit(app);
            }
            ConnectionState::Disconnected { .. } => {
                log::trace!("Disconnected from TrackAudio");
                state.clear();
                state.emit(app);
            }
            ConnectionState::ReconnectFailed { .. } => {
                log::warn!("TrackAudio reconnect failed");
                state.clear();
                app.emit("radio:state", RadioState::Error).ok();
            }
        }
    }
}

#[async_trait::async_trait]
impl Radio for TrackAudioRadio {
    async fn transmit(&self, state: TransmissionState) -> Result<(), RadioError> {
        let active = match state {
            TransmissionState::Active if !self.active.swap(true, Ordering::Relaxed) => true,
            TransmissionState::Inactive if self.active.swap(false, Ordering::Relaxed) => false,
            _ => return Ok(()),
        };

        log::trace!("Setting transmission {state:?}, sending active {active}");

        self.client
            .api()
            .transmit(active, Some(Self::TRANSMIT_TIMEOUT))
            .await
            .map_err(|err| {
                if !matches!(err, TrackAudioError::Timeout) {
                    self.app.emit("radio:state", RadioState::Error).ok();
                }
                RadioError::Transmit(format!("Failed to transmit via TrackAudio: {err}"))
            })?;

        Ok(())
    }

    async fn reconnect(&self) -> Result<(), RadioError> {
        self.state.clear();
        self.state.emit(&self.app);
        self.client.reconnect().map_err(|err| {
            self.app.emit("radio:state", RadioState::Error).ok();
            RadioError::Integration(format!("Failed to reconnect to TrackAudio: {err}"))
        })?;
        Ok(())
    }

    fn state(&self) -> RadioState {
        self.state.as_ref().into()
    }
}

impl Debug for TrackAudioRadio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackAudioRadio")
            .field("active", &self.active)
            .field("state", &self.state)
            .field("client", &self.client)
            .finish()
    }
}

impl Drop for TrackAudioRadio {
    fn drop(&mut self) {
        log::debug!("Dropping TrackAudioRadio");

        if self.active.load(Ordering::Relaxed)
            && let Err(err) =
                tauri::async_runtime::block_on(self.transmit(TransmissionState::Inactive))
        {
            log::warn!("Failed to set transmission Inactive while dropping: {err}");
        }

        self.state.clear();
        self.app.emit("radio:state", RadioState::NotConfigured).ok();

        self.cancellation_token.cancel();
    }
}

#[derive(Default)]
struct TrackAudioState {
    connected: AtomicBool,
    voice_connected: AtomicBool,
    transmitting: AtomicBool,
    receiving: AtomicBool,
    stations: RwLock<HashMap<String, bool>>,
}

impl From<&TrackAudioState> for RadioState {
    fn from(value: &TrackAudioState) -> Self {
        if !value.connected.load(Ordering::Relaxed) {
            return RadioState::Disconnected;
        }

        if !value.voice_connected.load(Ordering::Relaxed) {
            return RadioState::Connected;
        }

        // Priority: TX > RX > Idle
        if value.transmitting.load(Ordering::Relaxed) {
            return RadioState::TxActive;
        }

        if value.receiving.load(Ordering::Relaxed) {
            return RadioState::RxActive;
        }

        if value.stations.read().values().any(|&rx| rx) {
            return RadioState::RxIdle;
        }

        RadioState::VoiceConnected
    }
}

impl From<TrackAudioState> for RadioState {
    fn from(value: TrackAudioState) -> Self {
        Self::from(&value)
    }
}

impl Debug for TrackAudioState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackAudioState")
            .field("connected", &self.connected)
            .field("voice_connected", &self.voice_connected)
            .field("transmitting", &self.transmitting)
            .field("receiving", &self.receiving)
            .field("stations", &self.stations.read().len())
            .finish()
    }
}

impl TrackAudioState {
    fn emit(&self, app: &AppHandle) {
        RadioState::from(self).emit(app);
    }

    fn clear(&self) {
        self.connected.store(false, Ordering::Relaxed);
        self.voice_connected.store(false, Ordering::Relaxed);
        self.transmitting.store(false, Ordering::Relaxed);
        self.receiving.store(false, Ordering::Relaxed);
        self.stations.write().clear();
    }

    fn set_transmitting(&self, active: bool, app: &AppHandle) {
        self.transmitting.store(active, Ordering::Relaxed);
        self.emit(app);
    }

    fn set_receiving(&self, active: bool, app: &AppHandle) {
        self.receiving.store(active, Ordering::Relaxed);
        self.emit(app);
    }

    fn set_voice_connected(&self, connected: bool, app: &AppHandle) {
        self.voice_connected.store(connected, Ordering::Relaxed);
        self.emit(app);
    }

    fn set_connected(&self, connected: bool, app: &AppHandle) {
        self.connected.store(connected, Ordering::Relaxed);
        self.emit(app);
    }

    fn add_station(&self, callsign: String, app: &AppHandle) {
        {
            self.stations.write().insert(callsign, false);
        }

        self.emit(app);
    }

    fn update_station(
        &self,
        callsign: String,
        rx: Option<bool>,
        is_available: bool,
        app: &AppHandle,
    ) {
        {
            let mut stations = self.stations.write();
            if !is_available {
                stations.remove(&callsign);
            } else if let Some(rx) = rx {
                stations.insert(callsign, rx);
            } else {
                stations.entry(callsign).or_insert(false);
            }
        }

        self.emit(app);
    }

    fn sync_stations(&self, station_states: Vec<StationState>, app: &AppHandle) {
        {
            let mut stations = self.stations.write();
            stations.clear();

            for station_state in station_states {
                if station_state.is_available
                    && let Some(callsign) = station_state.callsign
                {
                    stations.insert(callsign, station_state.rx.unwrap_or(false));
                }
            }
        }

        self.emit(app);
    }
}
