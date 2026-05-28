use crate::app::state::AppState;
use crate::audio::manager::AudioManagerHandle;
use crate::config::{CLIENT_SETTINGS_FILE_NAME, Persistable, PersistedClientConfig};
use crate::error::Error;
use crate::radio::track_audio::TrackAudioRadioHandle;
use crate::replay::ClipMeta;
use crate::replay::recorder::{CLIP_PROGRESS_EVENT, ReplayRecorderHandle};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use vacs_audio::sources::wav::WavSource;

// TODO: Do we need some sort of a status?
// TODO: Fix me being unhappy with this entire file
// TODO: Fix me being unhappy with the function names/fields introduced in c1dd121016949dbde33800c830eb177b96701bea and 873627c29897f5bf6df985044f4c297d9c933510 (add skip and rewind)

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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayDevice {
    Headset,
    Speaker,
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn replay_play(
    app: AppHandle,
    recorder: State<'_, ReplayRecorderHandle>,
    app_state: State<'_, AppState>,
    audio_manager: State<'_, AudioManagerHandle>,
    id: u64,
    device: ReplayDevice,
) -> Result<(), Error> {
    take_and_stop_playing_source(&recorder, &audio_manager);

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
    let is_speaker = device == ReplayDevice::Speaker;

    let source_id = audio_manager.add_audio_source(
        move |sample_rate, channels| {
            Box::new(
                WavSource::from_file(
                    path,
                    sample_rate,
                    channels as usize,
                    volume,
                    None,
                    Some(Box::new(move |progress| {
                        app.emit(CLIP_PROGRESS_EVENT, progress).ok();
                        if progress == 1.0 {
                            let recorder = app.state::<ReplayRecorderHandle>();
                            if let Some(r) = recorder.write().as_mut() {
                                r.set_playing_source_id(None)
                            }
                        }
                    })),
                )
                .unwrap(),
            )
        },
        is_speaker,
    );
    audio_manager.start_audio_source(source_id, is_speaker);

    if let Some(r) = recorder.write().as_mut() {
        r.set_playing_source_id(Some((source_id, is_speaker)))
    }

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn replay_stop(
    app: AppHandle,
    recorder: State<'_, ReplayRecorderHandle>,
    audio_manager: State<'_, AudioManagerHandle>,
) -> Result<(), Error> {
    if !take_and_stop_playing_source(&recorder, &audio_manager) {
        log::warn!("replay stop called but no clip is playing");
    }

    app.emit(CLIP_PROGRESS_EVENT, 0.0).ok();

    Ok(())
}

fn take_and_stop_playing_source(
    recorder: &State<ReplayRecorderHandle>,
    audio_manager: &State<AudioManagerHandle>,
) -> bool {
    if let Some((source_id, is_speaker)) = recorder
        .write()
        .as_mut()
        .and_then(|r| r.take_playing_source_id())
    {
        audio_manager
            .read()
            .remove_audio_source(source_id, is_speaker);
        return true;
    }
    false
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn replay_skip(
    recorder: State<'_, ReplayRecorderHandle>,
    audio_manager: State<'_, AudioManagerHandle>,
    millis: u64,
) -> Result<(), Error> {
    if let Some((source_id, is_speaker)) = recorder
        .write()
        .as_mut()
        .and_then(|r| r.get_playing_source_id())
    {
        audio_manager.read().skip_in_audio_source(
            source_id,
            Duration::from_millis(millis),
            is_speaker,
        );
    } else {
        log::warn!("replay skip called but no clip is playing");
    }

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn replay_rewind(
    recorder: State<'_, ReplayRecorderHandle>,
    audio_manager: State<'_, AudioManagerHandle>,
    millis: u64,
) -> Result<(), Error> {
    if let Some((source_id, is_speaker)) = recorder
        .write()
        .as_mut()
        .and_then(|r| r.get_playing_source_id())
    {
        audio_manager.read().rewind_in_audio_source(
            source_id,
            Duration::from_millis(millis),
            is_speaker,
        );
    } else {
        log::warn!("replay skip called but no clip is playing");
    }

    Ok(())
}

/// Copy a clip to the saved directory within the app data dir. Saved clips are exempt
/// from rolling-deque eviction. Returns the destination path.
#[tauri::command]
#[vacs_macros::log_err]
pub async fn replay_export(
    app: AppHandle,
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

    if let Err(err) = app.opener().open_path(path.to_string_lossy(), None::<&str>) {
        return Err(Error::Other(Box::new(anyhow::anyhow!(
            "cannot open file: {}",
            err
        ))));
    }

    Ok(path)
}
