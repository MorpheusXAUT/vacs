use crate::app::state::AppState;
use crate::audio::manager::AudioManagerHandle;
use crate::config::{CLIENT_SETTINGS_FILE_NAME, Persistable, PersistedClientConfig};
use crate::error::Error;
use crate::radio::track_audio::TrackAudioRadioHandle;
use crate::replay::ClipMeta;
use crate::replay::recorder::ReplayRecorderHandle;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};
use vacs_audio::sources::wav::WavSource;

#[tauri::command]
#[vacs_macros::log_err]
pub async fn replay_get_enabled(app_state: State<'_, AppState>) -> Result<bool, Error> {
    Ok(app_state.lock().await.config.client.replay.enabled)
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn replay_set_enabled(
    app: AppHandle,
    app_state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), Error> {
    let (persisted_client_config, replay_config) = {
        let mut state = app_state.lock().await;

        if state.config.client.replay.enabled == enabled {
            return Ok(());
        }

        state.config.client.replay.enabled = enabled;
        let replay_config = state.config.client.replay.clone();
        (
            PersistedClientConfig::from(state.config.client.clone()),
            replay_config,
        )
    };

    let config_dir = app
        .path()
        .app_config_dir()
        .expect("Cannot get config directory");
    persisted_client_config.persist(&config_dir, CLIENT_SETTINGS_FILE_NAME)?;

    if enabled {
        // Start the recorder live if a TrackAudioRadio is currently active. If not, the
        // recorder will be started the next time the radio integration comes up.
        let radio = app.state::<TrackAudioRadioHandle>().read().clone();
        if let Some(radio) = radio {
            replay_config.start(&app, radio).await;
        } else {
            log::info!("replay enabled in config but no TrackAudio radio is active");
        }
    } else {
        // Stop any currently running recorder. The slot stays in place; future
        // ReplayConfig::start calls will be no-ops while replay is disabled.
        let handle = app.state::<ReplayRecorderHandle>();
        let existing = handle.write().take();
        if let Some(recorder) = existing {
            recorder.shutdown();
            log::info!("replay disabled; stopped active recorder");
        }
    }

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn replay_list(
    recorder: State<'_, ReplayRecorderHandle>,
) -> Result<Vec<ClipMeta>, Error> {
    Ok(recorder
        .read()
        .as_ref()
        .map(|r| r.list())
        .unwrap_or_default())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn replay_delete(
    recorder: State<'_, ReplayRecorderHandle>,
    id: u64,
) -> Result<bool, Error> {
    let Some(deleted) = recorder.read().as_ref().map(|r| r.delete(id)).transpose()? else {
        return Ok(false);
    };
    Ok(deleted)
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn replay_clear(recorder: State<'_, ReplayRecorderHandle>) -> Result<(), Error> {
    if let Some(r) = recorder.read().as_ref() {
        r.clear()?;
    }
    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn replay_play(
    recorder: State<'_, ReplayRecorderHandle>,
    app_state: State<'_, AppState>,
    audio_manager: State<'_, AudioManagerHandle>,
    id: u64,
) -> Result<(), Error> {
    if let Some(source_id) = recorder
        .read()
        .as_ref()
        .and_then(|r| r.get_playing_source_id())
    {
        audio_manager.read().remove_audio_source(source_id, false);
    }

    let path: Option<PathBuf> = recorder
        .read()
        .as_ref()
        .and_then(|r| r.get(id).map(|m| m.path));
    let Some(path) = path else {
        return Err(Error::Other(Box::new(anyhow::anyhow!(
            "clip {id} not found"
        ))));
    };

    let volume = {
        let state = app_state.lock().await;
        state.config.audio.output_device_volume
    };

    let audio_manager = audio_manager.read();

    let source_id = audio_manager.add_audio_source(
        move |sample_rate, channels| {
            Box::new(WavSource::from_file(path, sample_rate, channels as usize, volume).unwrap())
        },
        false,
    );
    audio_manager.start_audio_source(source_id, false);

    if let Some(r) = recorder.write().as_mut() {
        r.set_playing_source_id(Some(source_id))
    }

    Ok(())
}

/// Copy a clip to the saved directory within the app data dir. Saved clips are exempt
/// from rolling-deque eviction. Returns the destination path.
#[tauri::command]
#[vacs_macros::log_err]
pub async fn replay_export(
    recorder: State<'_, ReplayRecorderHandle>,
    id: u64,
) -> Result<PathBuf, Error> {
    let Some(path) = recorder
        .read()
        .as_ref()
        .map(|r| r.export(id, None))
        .transpose()?
    else {
        return Err(Error::Other(Box::new(anyhow::anyhow!(
            "recorder not running"
        ))));
    };
    Ok(path)
}
