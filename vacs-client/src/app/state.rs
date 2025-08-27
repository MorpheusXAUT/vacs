pub(crate) mod audio;
pub(crate) mod http;
mod sealed;
pub(crate) mod signaling;
pub(crate) mod webrtc;

use crate::app::state::webrtc::Call;
use crate::audio::manager::AudioManager;
use crate::config::{
    AppConfig, Persistable, PersistedAudioConfig, APP_USER_AGENT, AUDIO_SETTINGS_FILE_NAME,
};
use crate::secrets::cookies::SecureCookieStore;
use crate::signaling::Connection;
use anyhow::Context;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
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
    pub fn new(config_dir: PathBuf, data_dir: PathBuf) -> anyhow::Result<Self> {
        let cookie_store = Arc::new(
            SecureCookieStore::new(data_dir.join(".cookies"))
                .context("Failed to create secure cookie store")?,
        );
        let config = AppConfig::parse(&config_dir)?;

        // TODO handle is_fallback and update config accordingly

        let audio_manager = match AudioManager::new(&config.audio) {
            Ok(audio_manager) => audio_manager,
            Err(err) => {
                log::warn!(
                    "Failed to initialize audio manager with read config, falling back to default output device. Error: {err:?}"
                );

                let mut audio_config = config.audio.clone();
                audio_config.output_device_name = None;

                let audio_manager = AudioManager::new(&audio_config)?;

                log::info!(
                    "Audio manager initialized with fallback default output device. Persisting new audio config."
                );
                let persisted_audio_config: PersistedAudioConfig = audio_config.into();
                persisted_audio_config.persist(&config_dir, AUDIO_SETTINGS_FILE_NAME)?;

                audio_manager
            }
        };

        Ok(Self {
            config: config.clone(),
            connection: Connection::new(),
            audio_manager,
            http_client: reqwest::ClientBuilder::new()
                .user_agent(APP_USER_AGENT)
                .cookie_provider(cookie_store.clone())
                .timeout(Duration::from_millis(config.backend.timeout_ms))
                .build()
                .context("Failed to build HTTP client")?,
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
