use crate::radio::{Radio, RadioError, RadioState, TransmissionState};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use trackaudio::{ClientEvent, ConnectionState, TrackAudioClient};

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
        let client = match endpoint {
            Some(endpoint) => TrackAudioClient::connect_url(endpoint).await,
            None => TrackAudioClient::connect_default().await,
        }
        .map_err(|err| {
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
                    use trackaudio::Event;

                    match result {
                        Ok(Event::TxBegin(_)) => {
                            log::trace!("TrackAudio transmission started");
                            state.transmitting.store(true, Ordering::Relaxed);
                            state.emit(&app);
                        }
                        Ok(Event::TxEnd(_)) => {
                            log::trace!("TrackAudio transmission ended");
                            state.transmitting.store(false, Ordering::Relaxed);
                            state.emit(&app);
                        }
                        Ok(Event::RxBegin(_)) => {
                            log::trace!("TrackAudio reception started");
                            state.receiving.store(true, Ordering::Relaxed);
                            state.emit(&app);
                        }
                        Ok(Event::RxEnd(_)) => {
                            log::trace!("TrackAudio reception ended");
                            state.receiving.store(false, Ordering::Relaxed);
                            state.emit(&app);
                        }
                        Ok(Event::VoiceConnectedState(payload)) => {
                            log::trace!("TrackAudio voice connection state changed: {payload:?}");
                            state.voice_connected.store(payload.connected, Ordering::Relaxed);
                            state.emit(&app);
                        }
                        Ok(Event::Client(ClientEvent::ConnectionStateChanged(connection_state))) => {
                            match connection_state {
                                ConnectionState::Connected => {
                                    log::trace!("Successfully connected to TrackAudio");
                                    state.connected.store(true, Ordering::Relaxed);

                                    let api = client.api();
                                    let voice_connected = api
                                        .get_voice_connected_state(Some(Self::VOICE_CONNECTED_STATE_TIMEOUT))
                                        .await
                                        .unwrap_or(false);
                                    state
                                        .voice_connected
                                        .store(voice_connected, Ordering::Relaxed);

                                    let station_states = api.get_station_states(Some(Self::STATION_STATES_TIMEOUT)).await.unwrap_or_default();
                                    {
                                        let mut stations = state.stations.write();
                                        stations.clear();
                                        for station_state in station_states {
                                            if let Some(callsign) = station_state.callsign {
                                                stations.insert(callsign, station_state.rx.unwrap_or(false));
                                            }
                                        }
                                    }

                                    state.emit(&app);
                                }
                                ConnectionState::Connecting { .. }
                                | ConnectionState::Reconnecting { .. } => {
                                    log::trace!("Connecting to TrackAudio");
                                    state.clear();
                                    state.emit(&app);
                                }
                                ConnectionState::Disconnected { .. } => {
                                    log::trace!("Disconnected from TrackAudio");
                                    state.clear();
                                    state.emit(&app);
                                }
                                ConnectionState::ReconnectFailed { .. } => {
                                    log::warn!("TrackAudio reconnect failed");
                                    state.clear();
                                    app.emit("radio:state", RadioState::Error).ok();
                                }
                            }
                        }
                        Ok(Event::Client(ClientEvent::CommandSendFailed { .. }))
                        | Ok(Event::Client(ClientEvent::EventDeserializationFailed { .. })) => {
                            log::warn!("TrackAudio client error event");
                            state.clear();
                            app.emit("radio:state", RadioState::Error).ok();
                        }
                        Ok(Event::StationAdded(payload)) => {
                            log::trace!("TrackAudio station added: {}", payload.callsign);
                            state
                                .stations
                                .write()
                                .insert(payload.callsign, false);
                            state.emit(&app);
                        }
                        Ok(Event::StationStateUpdate(payload)) => {
                            log::trace!("TrackAudio station state update: {payload:?}");
                            if let Some(callsign) = payload.callsign {
                                let mut stations = state.stations.write();

                                if !payload.is_available {
                                    stations.remove(&callsign);
                                } else if let Some(rx) = payload.rx {
                                    stations.insert(callsign, rx);
                                } else {
                                    stations.entry(callsign).or_insert(false);
                                }
                            }
                            state.emit(&app);
                        }
                        Ok(Event::StationStates(payload)) => {
                            log::trace!(
                                "Received full station states list for {} stations",
                                payload.stations.len()
                            );
                            let mut stations = state.stations.write();
                            stations.clear();
                            for envelope in payload.stations {
                                if envelope.value.is_available &&
                                    let Some(callsign) = envelope.value.callsign {
                                    stations.insert(callsign, envelope.value.rx.unwrap_or(false));
                                }
                            }
                            state.emit(&app);
                        }
                        Ok(event) => {
                            log::trace!("Received TrackAudio event: {event:?}");
                        }
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
                self.app.emit("radio:state", RadioState::Error).ok();
                RadioError::Transmit(format!("Failed to transmit via TrackAudio: {err}"))
            })?;

        Ok(())
    }

    async fn reconnect(&self) -> Result<(), RadioError> {
        self.state.clear();
        self.state.emit(&self.app);
        self.client.reconnect().await.map_err(|err| {
            RadioError::Integration(format!("Failed to reconnect to TrackAudio: {err}"))
        })?;
        Ok(())
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
        if !value.connected.load(Ordering::Relaxed)
            || !value.voice_connected.load(Ordering::Relaxed)
        {
            return RadioState::Disconnected;
        }

        // Priority: TX > RX > Idle
        if value.transmitting.load(Ordering::Relaxed) {
            return RadioState::TxActive;
        }

        if value.receiving.load(Ordering::Relaxed) {
            return RadioState::RxActive;
        }

        if value.stations.read().values().any(|&rx| rx) {
            RadioState::RxIdle
        } else {
            RadioState::Connected
        }
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

impl From<TrackAudioState> for RadioState {
    fn from(value: TrackAudioState) -> Self {
        Self::from(&value)
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
}
