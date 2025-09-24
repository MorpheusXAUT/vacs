pub(crate) mod audio;
pub(crate) mod http;
mod sealed;
pub(crate) mod signaling;
pub(crate) mod webrtc;

use crate::app::state::webrtc::Call;
use crate::audio::manager::AudioManager;
use crate::config::AppConfig;
use crate::error::{StartupError, StartupErrorExt};
use crate::signaling::Connection;
use std::collections::{HashMap, HashSet};
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub struct AppStateInner {
    pub config: AppConfig,
    shutdown_token: CancellationToken,
    connection: Connection,
    audio_manager: AudioManager,
    active_call: Option<Call>,
    held_calls: HashMap<String, Call>,       // peer_id -> call
    outgoing_call_peer_id: Option<String>,   // peer_id
    incoming_call_peer_ids: HashSet<String>, // peer_id
}

pub type AppState = Mutex<AppStateInner>;

impl AppStateInner {
    pub fn new(app: &AppHandle) -> Result<Self, StartupError> {
        let config_dir = app
            .path()
            .app_config_dir()
            .map_startup_err(StartupError::Config)?;

        let config = AppConfig::parse(&config_dir).map_startup_err(StartupError::Config)?;
        let shutdown_token = CancellationToken::new();

        Ok(Self {
            config: config.clone(),
            connection: Connection::new(
                app.clone(),
                shutdown_token.child_token(),
                &config.backend.ws_url,
                config.client.max_signaling_reconnect_attempts(),
            ),
            shutdown_token,
            audio_manager: AudioManager::new(app.clone(), &config.audio)
                .map_startup_err(StartupError::Audio)?,
            active_call: None,
            held_calls: HashMap::new(),
            outgoing_call_peer_id: None,
            incoming_call_peer_ids: HashSet::new(),
        })
    }

    pub fn shutdown(&self) {
        self.shutdown_token.cancel();
    }
}
