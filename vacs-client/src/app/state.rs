pub(crate) mod audio;
pub(crate) mod http;
mod sealed;
pub(crate) mod signaling;
pub(crate) mod webrtc;

use crate::app::state::webrtc::Call;
use crate::audio::manager::AudioManager;
use crate::config::{APP_USER_AGENT, AppConfig};
use crate::error::{StartupError, StartupErrorExt};
use crate::secrets::cookies::SecureCookieStore;
use crate::signaling::Connection;
use anyhow::Context;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

pub struct AppStateInner {
    pub config: AppConfig,
    connection: Connection,
    audio_manager: AudioManager,
    pub http_client: reqwest::Client,
    cookie_store: Arc<SecureCookieStore>,
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
        let data_dir = app
            .path()
            .app_data_dir()
            .map_startup_err(StartupError::Config)?;

        let cookie_store = Arc::new(
            SecureCookieStore::new(data_dir.join(".cookies"))
                .context("Failed to create secure cookie store")
                .map_startup_err(StartupError::Config)?,
        );
        let config = AppConfig::parse(&config_dir).map_startup_err(StartupError::Config)?;

        Ok(Self {
            config: config.clone(),
            connection: Connection::new(config.client.max_signaling_reconnect_attempts()),
            audio_manager: AudioManager::new(app.clone(), &config.audio)
                .map_startup_err(StartupError::Audio)?,
            http_client: reqwest::ClientBuilder::new()
                .user_agent(APP_USER_AGENT)
                .cookie_provider(cookie_store.clone())
                .timeout(Duration::from_millis(config.backend.timeout_ms))
                .build()
                .context("Failed to build HTTP client")
                .map_startup_err(StartupError::Other)?,
            cookie_store,
            active_call: None,
            held_calls: HashMap::new(),
            outgoing_call_peer_id: None,
            incoming_call_peer_ids: HashSet::new(),
        })
    }

    pub fn persist(&self) -> anyhow::Result<()> {
        self.cookie_store
            .save()
            .context("Failed to save cookie store")?;

        Ok(())
    }
}
